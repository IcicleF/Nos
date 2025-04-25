use std::array;
use std::cell::UnsafeCell;
use std::sync::{atomic::*, Arc, Barrier};
use std::thread;
use std::{mem, ptr, slice};

#[cfg(feature = "use_ahash")]
use ahash::RandomState;
#[cfg(feature = "use_fxhash")]
use fxhash::FxBuildHasher as RandomState;
#[cfg(not(any(feature = "use_ahash", feature = "use_fxhash")))]
use std::collections::hash_map::RandomState;

use anyhow::{Context as _, Result};
use clap::Parser;
use dashmap::DashMap;
use futures::{
    executor::block_on,
    future::{join_all, select_all},
};
use local_ip_address::local_ip;
use rand::prelude::*;
use rrppcc::{type_alias::*, *};

#[allow(unused_imports)]
use quanta::Instant;

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use nos::ctrl::*;
use nos::ec::EcConfig;
use nos::objstore::{ObjMeta, ObjStoreStats, RpcRes, MAX_VALUE_SIZE};
use nos::rpc_types::*;
use nos::{Args, Key};

thread_local! {
    /// RPC resources for the current thread.
    static RESOURCES: UnsafeCell<Option<RpcRes>> = const { UnsafeCell::new(None) };
}

/// A value split.
#[derive(Clone)]
struct ValueSplit {
    /// The original length of the value.
    len: usize,

    /// The value split.
    /// The length of this split should be `ceil(len / k)`.
    value: Box<[u8]>,
}

impl ValueSplit {
    fn new(len: usize, value: Box<[u8]>) -> Self {
        Self { len, value }
    }
}

/// A local object store.
///
/// EC-Split baseline needs no special logic.
/// User just write many pieces of a key-value entry together.
struct ObjStore<K>
where
    K: Key,
{
    /// Cluster information. Specified in `new` by the user.
    cluster: Cluster,

    /// EC config. Specified in `new` by the user.
    ec_config: EcConfig,

    /// Real object key-value store. Allocated in `new`.
    store: DashMap<K, ValueSplit, RandomState>,

    /// Flag for halting the object store's listener threads.
    halt: AtomicBool,

    /// Performance statistics.
    #[allow(dead_code)]
    stats: ObjStoreStats,
}

/// It should be safe to access `ObjStore` across threads.
unsafe impl<K> Send for ObjStore<K> where K: Key {}
unsafe impl<K> Sync for ObjStore<K> where K: Key {}

impl<K> ObjStore<K>
where
    K: Key,
{
    /// Initial capacity of the key-value store.
    pub const INITIAL_MAP_CAPACITY: usize = 100_000;

    /// Create a RAID baseline object store instance.
    fn new(cluster: &Cluster, ec_config: &EcConfig) -> Self {
        Self {
            cluster: cluster.clone(),
            ec_config: ec_config.clone(),
            store: DashMap::with_capacity_and_hasher(
                Self::INITIAL_MAP_CAPACITY,
                RandomState::default(),
            ),
            halt: AtomicBool::new(false),
            stats: ObjStoreStats::default(),
        }
    }

    /// Notify all the listener threads to stop.
    fn halt(&self) {
        self.halt.store(true, Ordering::SeqCst);
    }

    /// Get whether this object store is halted.
    #[inline]
    fn is_halted(&self) -> bool {
        self.halt.load(Ordering::Relaxed)
    }

    /// Insert the given object into the object store.
    /// This node must be the primary node of the object.
    ///
    /// The length should be the full length of the value, not of the split
    /// being inserted.
    #[inline]
    fn insert(&self, key: K, split: Box<[u8]>, len: usize) {
        self.store.insert(key, ValueSplit::new(len, split));
    }

    /// Retrieve the given object from the object store.
    #[inline]
    fn get(&self, key: K) -> Option<ValueSplit> {
        self.store.get(&key).map(|entry| entry.value().clone())
    }

    /// Retrieve the given object from the object store and copy it into the
    /// given buffer for further RDMA transmission.
    ///
    /// The returned length is the full length of the value, not of the split
    /// being retrieved.
    #[inline]
    unsafe fn get_to(&self, key: K, split: *mut u8) -> usize {
        let entry = self.store.get(&key);
        if let Some(entry) = entry {
            let entry = entry.value();
            unsafe {
                std::ptr::copy_nonoverlapping(entry.value.as_ptr(), split, entry.value.len())
            };
            entry.len
        } else {
            0
        }
    }

    /// Request handler callback.
    #[inline]
    pub async fn rpc_handler(&self, req: RequestHandle) -> MsgBuf {
        let rpc = req.rpc();

        // Check request sanity
        debug_assert!(
            (BEGIN_DATA_RPC..=END_DATA_RPC).contains(&req.req_type()),
            "RPC type {} incorrectly handled by `rpc_handler`",
            req.req_type()
        );

        // Get request header and determine request type
        let req_buf = req.req_buf();
        debug_assert!(
            req_buf.len() >= mem::size_of::<ObjMeta<K>>(),
            "request too short ({} bytes) to contain a full header",
            req_buf.len()
        );

        // SAFETY:
        // - Clients definitely send a header at the beginning of the request
        // - Data validity is ensured by NIC
        // - There is enough space for the header (checked by the assertion)
        let hdr = unsafe { <ObjMeta<K>>::from_ptr(req_buf.as_ptr()) };
        let stripe_width = self.ec_config.k + self.ec_config.p;
        let primary = (hdr.key % self.cluster.size() as u64) as usize;

        if req.req_type() == RPC_DATA_WRITE && hdr.len == 0 {
            // Empty write. Currently, this shouldn't happen.
            log::warn!(
                "{}: received empty write to object {} (primary {})",
                self.cluster.rank(),
                hdr.key,
                primary
            );

            let mut resp_buf = req.pre_resp_buf();
            let retval = resp_buf.as_ptr() as *mut u64;
            unsafe { *retval = 0 };
            resp_buf.set_len(mem::size_of::<u64>());
            return resp_buf;
        }

        // Response length
        let mut resp_payload_len = 0;

        // If this is a write and I am the primary, then it is a full write.
        // I need to split into chunks, compute a stripe, and send it to the
        // participants, i.e., (k + p - 1) nodes after me.
        if req.req_type() == RPC_DATA_WRITE && primary == self.cluster.rank() {
            assert!(
                req_buf.len() >= <ObjMeta<K>>::len() + hdr.len as usize,
                "request too short: {} vs {} (header 16 + payload {})",
                req_buf.len(),
                <ObjMeta<K>>::len() + hdr.len as usize,
                hdr.len
            );
            let chunk_len = (hdr.len as usize - 1) / self.ec_config.k + 1;

            // let (mut t_coding, mut t_repl, mut t_kvs) = (0u64, 0u64, 0u64);
            // let start_time = Instant::now();

            // 1. Allocate message buffers.
            let mut req_bufs = (0..stripe_width)
                .map(|_| rpc.alloc_msgbuf(<ObjMeta<K>>::len() + chunk_len))
                .collect::<Vec<_>>();
            let mut resp_bufs = (0..stripe_width)
                .map(|_| rpc.alloc_msgbuf(mem::size_of::<u64>() * 2))
                .collect::<Vec<_>>();

            // let t1 = Instant::now();
            // 2. Construct the stripe in the allocated buffers.
            {
                // Copy the data chunks.
                for (i, nested_req_buf) in req_bufs.iter_mut().enumerate().take(self.ec_config.k) {
                    unsafe {
                        // The last chunk may be shorter than the others, so it needs trailing zeros.
                        // For simplicity, we just zero-initialize it.
                        ptr::write_bytes(nested_req_buf.as_mut_ptr(), 0, nested_req_buf.len());

                        // Copy the chunk into the buffer.
                        ptr::copy_nonoverlapping(
                            req_buf
                                .as_ptr()
                                .add(mem::size_of::<ObjMeta<K>>() + i * chunk_len),
                            nested_req_buf.as_mut_ptr().add(<ObjMeta<K>>::len()),
                            chunk_len.min(hdr.len as usize - i * chunk_len),
                        );
                    };
                }

                // Encode the stripe.
                let data_chunk_refs = (0..self.ec_config.k)
                    .map(|i| unsafe {
                        slice::from_raw_parts(
                            req_bufs[i].as_ptr().add(<ObjMeta<K>>::len()),
                            chunk_len,
                        )
                    })
                    .collect::<Vec<_>>();
                let mut parity_chunk_refs = (self.ec_config.k..stripe_width)
                    .map(|i| unsafe {
                        slice::from_raw_parts_mut(
                            req_bufs[i].as_mut_ptr().add(<ObjMeta<K>>::len()),
                            chunk_len,
                        )
                    })
                    .collect::<Vec<_>>();
                self.ec_config
                    .encode(&data_chunk_refs, &mut parity_chunk_refs);
            }

            // let t2 = Instant::now();
            // t_coding += (t2 - t1).as_nanos() as u64;

            // 3. Fill in headers.
            for nested_req_buf in &req_bufs {
                unsafe { ptr::write_unaligned(nested_req_buf.as_ptr() as *mut ObjMeta<K>, hdr) };
            }

            // 4. Prepare sessions.
            let sessions = (1..stripe_width)
                .map(|i| {
                    let node_id = (self.cluster.rank() + i) % self.cluster.size();
                    debug_assert_ne!(node_id, self.cluster.rank(), "replicating to self");
                    RESOURCES.with(|res| {
                        let res = unsafe { &*res.get() }.as_ref().unwrap();
                        res.session_to(rpc, node_id)
                    })
                })
                .collect::<Vec<_>>();

            // 5. Make async requests.
            let mut repl_futs = Vec::with_capacity(stripe_width - 1);
            let mut resp_buf_slice = &mut resp_bufs[1..];
            for (sess, nested_req_buf) in sessions.iter().zip(req_bufs.iter().skip(1)) {
                let nested_resp_buf = {
                    let (head, tail) = resp_buf_slice.split_first_mut().unwrap();
                    resp_buf_slice = tail;
                    head
                };
                repl_futs.push(sess.request(RPC_DATA_WRITE, nested_req_buf, nested_resp_buf));
            }

            // let t3 = Instant::now();
            // t_repl += (t3 - t2).as_nanos() as u64;

            // 6. Insert locally to hide some latency.
            let my_split = unsafe {
                let mut v = Vec::with_capacity(chunk_len);
                ptr::copy_nonoverlapping(
                    req_bufs[0].as_ptr().add(<ObjMeta<K>>::len()),
                    v.as_mut_ptr(),
                    chunk_len,
                );
                v.set_len(chunk_len);
                v.into_boxed_slice()
            };
            self.insert(hdr.key, my_split, hdr.len as usize);

            // let t4 = Instant::now();
            // t_kvs += (t4 - t3).as_nanos() as u64;

            // 7. Concurrently wait for all replication requests to finish,=.
            join_all(repl_futs).await;

            // t_repl += (Instant::now() - t4).as_nanos() as u64;

            // let total_time = start_time.elapsed().as_nanos() as u64;
            // self.stats.record(
            //     t_coding,
            //     t_repl,
            //     t_kvs,
            //     total_time - t_coding - t_repl - t_kvs,
            // );
        }

        // If this is a write and I am not the primary, then it is a chunk write.
        // I just need to insert the chunk into the local store.
        if req.req_type() == RPC_DATA_WRITE && primary != self.cluster.rank() {
            let split = unsafe {
                let chunk_len = (hdr.len as usize - 1) / self.ec_config.k + 1;
                debug_assert!(
                    req_buf.len() >= <ObjMeta<K>>::len() + chunk_len,
                    "request too short: {} vs {} (header 16 + payload {} (computed from full len {}))",
                    req_buf.len(),
                    <ObjMeta<K>>::len() + chunk_len,
                    chunk_len, hdr.len
                );

                let mut v = Vec::with_capacity(chunk_len);
                ptr::copy_nonoverlapping(
                    req_buf.as_ptr().add(<ObjMeta<K>>::len()),
                    v.as_mut_ptr(),
                    chunk_len,
                );
                v.set_len(chunk_len);
                v.into_boxed_slice()
            };
            self.insert(hdr.key, split, hdr.len as usize);
        }

        // If this is a client read, then different from all other baselines
        // and Nos, I must contact many peers to serve it.
        let mut resp_buf = req.pre_resp_buf();
        if req.req_type() == RPC_DATA_READ {
            debug_assert!(
                (0..stripe_width)
                    .any(|i| (primary + i) % self.cluster.size() == self.cluster.rank()),
                "read request not routed to a node in the stripe"
            );

            // Query the local store for object length.
            let my_split = self.get(hdr.key);
            if let Some(my_split) = my_split {
                // Set expected response length
                resp_payload_len = my_split.len;

                // Compute split length, this will be used when reading from peers.
                let chunk_len = (resp_payload_len - 1) / self.ec_config.k + 1;
                debug_assert_eq!(
                    my_split.value.len(),
                    chunk_len,
                    "split length mismatch: got {}, expected {}",
                    my_split.value.len(),
                    chunk_len
                );

                // Late binding.
                const LATE_BINDING_NUM: usize = 1;
                assert!(LATE_BINDING_NUM <= self.ec_config.p);

                // 1. Allocate message buffers and prepare for sends.
                let nested_req_buf = rpc.alloc_msgbuf(<ObjMeta<K>>::len());
                unsafe {
                    ptr::write_unaligned(
                        nested_req_buf.as_ptr() as *mut ObjMeta<K>,
                        ObjMeta::new(hdr.key, 0, 0),
                    )
                };

                // For response buffers, we allocate some extra ones to prepare for possible recovery.
                let mut resp_bufs = (0..self.ec_config.width())
                    .map(|_| rpc.alloc_msgbuf(mem::size_of::<u64>() + chunk_len))
                    .collect::<Vec<_>>();

                // 2. Prepare sessions.
                let sessions = (0..stripe_width)
                    .map(|i| {
                        let node_id = (primary + i) % self.cluster.size();
                        if node_id == self.cluster.rank() {
                            None
                        } else {
                            Some(RESOURCES.with(|res| {
                                let res = unsafe { &*res.get() }.as_ref().unwrap();
                                res.session_to(rpc, node_id)
                            }))
                        }
                    })
                    .collect::<Vec<_>>();

                let mut all_srcs = (0..stripe_width).collect::<Vec<_>>();
                all_srcs.shuffle(&mut thread_rng());

                // 3. Request value splits from nodes in the stripe.
                let mut read_futs = Vec::with_capacity(self.ec_config.k + LATE_BINDING_NUM);
                let mut read_futs_idx = Vec::with_capacity(self.ec_config.k + LATE_BINDING_NUM);
                let mut received_split_src = Vec::with_capacity(self.ec_config.k);
                let mut resp_buf_slice = &mut resp_bufs[..];

                let mut cnt = 0;
                for idx in all_srcs.iter() {
                    let node_id = (primary + *idx) % self.cluster.size();
                    if !self.cluster[node_id].is_alive() {
                        continue;
                    }

                    // Always consume the slice.
                    let nested_resp_buf = {
                        let (head, tail) = resp_buf_slice.split_first_mut().unwrap();
                        resp_buf_slice = tail;
                        head
                    };

                    // Local splits can be directly used.
                    if node_id == self.cluster.rank() {
                        // Fill the corresponding response buffer as if it is an RPC response.
                        unsafe {
                            ptr::copy_nonoverlapping(
                                my_split.value.as_ptr(),
                                nested_resp_buf.as_mut_ptr().add(mem::size_of::<u64>()),
                                my_split.value.len(),
                            )
                        };
                        received_split_src.push(*idx);
                    } else {
                        let sess = sessions[*idx].as_ref().unwrap();
                        read_futs.push(sess.request(
                            RPC_DATA_SPLIT_READ,
                            &nested_req_buf,
                            nested_resp_buf,
                        ));
                        read_futs_idx.push(*idx);
                    }

                    cnt += 1;
                    if cnt == self.ec_config.k + LATE_BINDING_NUM {
                        break;
                    }
                }
                assert!(
                    cnt >= self.ec_config.k,
                    "not enough splits to reconstruct an object"
                );

                // 4. Wait for the first `k` requests to finish, abort the remainings.
                for _ in received_split_src.len()..self.ec_config.k {
                    let (_, idx, new_read_futs) = select_all(read_futs).await;
                    read_futs = new_read_futs;

                    // Record the index of the received split.
                    assert!(idx < read_futs_idx.len());
                    received_split_src.push(read_futs_idx[idx]);
                    read_futs_idx.remove(idx);
                }
                assert_eq!(received_split_src.len(), self.ec_config.k);

                for fut in read_futs {
                    fut.abort();
                }
                received_split_src.sort_unstable();

                // If necessary, perform recovery.
                if received_split_src.iter().any(|&i| i >= self.ec_config.k) {
                    // Prepare data and parity buffers.
                    let mut data = (0..self.ec_config.k)
                        .map(|i| unsafe {
                            slice::from_raw_parts_mut(
                                resp_bufs[i].as_mut_ptr().add(mem::size_of::<u64>()),
                                chunk_len,
                            )
                        })
                        .collect::<Vec<_>>();

                    let parity = received_split_src
                        .iter()
                        .filter(|&&i| i >= self.ec_config.k)
                        .map(|&i| unsafe {
                            slice::from_raw_parts(
                                resp_bufs[i].as_ptr().add(mem::size_of::<u64>()),
                                chunk_len,
                            )
                        })
                        .collect::<Vec<_>>();

                    // Perform recovery.
                    self.ec_config
                        .recover(&mut data, &parity, &received_split_src);
                }

                // All data chunks are in their correct places now.
                // Copy data splits into return buffer.
                for (i, nested_resp_buf) in resp_bufs.iter().enumerate().take(self.ec_config.k) {
                    let offset = chunk_len * i;
                    let len = if i + 1 == self.ec_config.k {
                        resp_payload_len - offset
                    } else {
                        chunk_len
                    };

                    // Copy the split into the response buffer
                    unsafe {
                        ptr::copy_nonoverlapping(
                            nested_resp_buf.as_ptr().add(mem::size_of::<u64>()),
                            resp_buf.as_mut_ptr().add(mem::size_of::<u64>() + offset),
                            len,
                        );
                    }
                }
            } else {
                let in_stripe = (0..stripe_width)
                    .any(|i| (primary + i) % self.cluster.size() == self.cluster.rank());

                // Redirect to a node that can serve the request if I am not in the stripe
                if !in_stripe {
                    let target = (0..stripe_width)
                        .map(|i| (primary + i) % self.cluster.size())
                        .find(|&i| self.cluster[i].is_alive())
                        .unwrap();
                    assert!(target != self.cluster.rank());

                    let nested_req_buf = rpc.alloc_msgbuf(<ObjMeta<K>>::len());
                    let mut nested_resp_buf =
                        rpc.alloc_msgbuf(mem::size_of::<u64>() + MAX_VALUE_SIZE);
                    unsafe {
                        ptr::write_unaligned(
                            nested_req_buf.as_ptr() as *mut ObjMeta<K>,
                            ObjMeta::new(hdr.key, 0, 0),
                        )
                    };

                    let sess = RESOURCES.with(|res| {
                        let res = unsafe { &*res.get() }.as_ref().unwrap();
                        res.session_to(rpc, target)
                    });
                    sess.request(RPC_DATA_READ, &nested_req_buf, &mut nested_resp_buf)
                        .await;

                    // This will get the whole object, copy it into the response buffer.
                    resp_payload_len = nested_resp_buf.len() - mem::size_of::<u64>();
                    unsafe {
                        ptr::copy_nonoverlapping(
                            nested_resp_buf.as_ptr().add(mem::size_of::<u64>()),
                            resp_buf.as_mut_ptr().add(mem::size_of::<u64>()),
                            resp_payload_len,
                        )
                    };
                } else {
                    resp_payload_len = 0;
                }
            }
        }

        // If this is a split read, then I just fetch it from the local store.
        // Return split length instead of the full length.
        if req.req_type() == RPC_DATA_SPLIT_READ {
            let len =
                unsafe { self.get_to(hdr.key, resp_buf.as_mut_ptr().add(mem::size_of::<u64>())) };
            resp_payload_len = if len == 0 {
                0
            } else {
                (len - 1) / self.ec_config.k + 1
            };
        }

        // Fill response value.
        let retval = resp_buf.as_ptr() as *mut u64;
        unsafe {
            *retval = resp_payload_len as u64;
        }
        resp_buf.set_len(mem::size_of::<u64>() + resp_payload_len);
        resp_buf
    }

    /// Control-plane RPC request handler callback.
    /// Different from data RPCs and recovery RPCs, control-plane RPCs return
    /// immediately with no return values.
    pub fn ctrl_rpc_handler(&self, req: &RequestHandle) {
        // Check request sanity
        debug_assert!(
            (BEGIN_CONTROL_RPC..=END_CONTROL_RPC).contains(&req.req_type()),
            "RPC type {} incorrectly handled by `ctrl_rpc_handler`",
            req.req_type()
        );

        // Get request header and determine request type
        let req_buf = req.req_buf();
        debug_assert!(
            req_buf.len() >= mem::size_of::<u64>(),
            "request too short ({} bytes) to contain a peer ID",
            req_buf.len()
        );

        if req.req_type() == RPC_HALT {
            self.halt();
        } else {
            let peer_id = unsafe { *(req_buf.as_ptr() as *const u64) } as usize;
            match req.req_type() {
                RPC_MAKE_ALIVE => self.cluster[peer_id].make_alive(),
                RPC_MAKE_FAILED => self.cluster[peer_id].make_failed(),
                RPC_REPORT_STATS => self.stats.report(),
                _ => {}
            };
        }
    }

    /// Prepare RPC resources for the current thread.
    ///
    /// # Safety
    ///
    /// - This method must be called once and only once.
    /// - This method must be called before running RPC server on the current thread.
    pub async unsafe fn prepare_rpc(&self, rpc: &Rpc, rpc_id: RpcId) {
        if RESOURCES.with(|res| unsafe { &*res.get() }.is_some()) {
            return;
        }
        let resource = RpcRes::new(rpc, &self.cluster, rpc_id).await;
        RESOURCES.with(move |res| (&mut *res.get()).insert(resource));
    }
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    ///////////////////////////////// Config /////////////////////////////////
    let cluster = Cluster::load_toml_file(&args.config_file, Role::Server(args.uuid))
        .with_context(|| "failed to load cluster config")?;
    log::info!("loaded cluster config, this is node {}", cluster.rank());

    let ec_config = EcConfig::load_toml_file(&args.config_file);
    let nthreads = NThreads::load_toml_file(&args.config_file)?;
    if cluster.rank() == 0 {
        log::info!("===== Split baseline =====");
        log::info!("EC config: {:?}", ec_config);
        log::info!("NThreads: {:#?}", nthreads);
    }

    let core_ids = core_affinity::get_core_ids()
        .unwrap()
        .into_iter()
        .filter(|core_id| core_id.id >= cluster.me().cpu())
        .collect::<Vec<_>>();

    // Bind to the specified core and enable local allocation
    let mut core_id_iter = core_ids.into_iter();
    core_affinity::set_for_current(core_id_iter.next().unwrap());

    numa_sys::set_localalloc();

    //////////////////////////////// ObjStore ////////////////////////////////
    // Initiate object store.
    let objstore = Arc::new(<ObjStore<u64>>::new(&cluster, &ec_config));

    // Initialize RPC.
    let nexus = Nexus::new((
        local_ip().expect("failed to get local IPv4 address"),
        cluster.me().port(),
    ));
    log::info!("Nexus working on {:?}", nexus.uri());

    // Spawn RPC handler threads
    let mut listener_handles = Vec::with_capacity(nthreads.rpc);

    // Two barriers used to sync RPC threads twice
    // Two barriers used to sync RPC threads twice
    let barriers: [_; 2] = array::from_fn(|_| Arc::new(Barrier::new(nthreads.rpc)));
    let rpc_created = Arc::new(AtomicUsize::new(nthreads.rpc));

    // Rpc ID starts from 1
    for id in 1..=nthreads.rpc {
        let nexus = nexus.clone();
        let objstore = objstore.clone();
        let core_id = core_id_iter.next().unwrap();
        let cluster = cluster.clone();

        let barriers = barriers.clone();
        let rpc_created = rpc_created.clone();

        listener_handles.push(thread::spawn(move || {
            // Bind core
            core_affinity::set_for_current(core_id);

            // Prepare RPC and wait for all RPC threads in the cluster to be ready
            let rpc = {
                let mut rpc = Rpc::new(&nexus, id as RpcId, NIC_NAME, NIC_PHYPORT);
                for rpc_type in ALL_RPC_TYPES {
                    let objstore = objstore.clone();
                    match *rpc_type {
                        ty if (BEGIN_DATA_RPC..=END_DATA_RPC).contains(&ty) => {
                            rpc.set_handler(ty, move |req| {
                                let objstore = objstore.clone();
                                async move { objstore.rpc_handler(req).await }
                            });
                        }
                        ty if (BEGIN_CONTROL_RPC..=END_CONTROL_RPC).contains(&ty) => {
                            rpc.set_handler(ty, move |req| {
                                objstore.ctrl_rpc_handler(&req);
                                async move {
                                    let mut resp = req.pre_resp_buf();
                                    resp.set_len(0);
                                    resp
                                }
                            });
                        }
                        _ => {}
                    };
                }
                rpc
            };
            {
                barriers[0].wait();
                if id == 1 {
                    DistBarrier::wait(&cluster);
                }
                barriers[1].wait();
            }

            // Connect to the RPCs
            // SAFETY: call only once, no RPC servers have been ran yet.
            block_on(unsafe { objstore.prepare_rpc(&rpc, id as RpcId) });

            // Do not barrier here, since this stops RPC event loop and results in deadlock.
            // Instead, continue to run the RPC server and let main thread barrier.
            rpc_created.fetch_sub(1, Ordering::SeqCst);

            // Run the RPC server
            const LOOP_BATCH: usize = 5000;
            while !objstore.is_halted() {
                for _ in 0..LOOP_BATCH {
                    rpc.progress();
                }
            }
        }));
    }
    log::debug!("spawned all handler threads");

    // Wait for all RPC threads to be ready
    while rpc_created.load(Ordering::SeqCst) != 0 {
        std::hint::spin_loop();
    }
    DistBarrier::wait(&cluster);
    if cluster.rank() == 0 {
        log::info!("===== cluster is ready! =====");
    }

    // Join handlers
    for handle in listener_handles {
        handle.join().unwrap();
    }
    Ok(())
}
