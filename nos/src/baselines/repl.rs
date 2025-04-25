use std::cell::UnsafeCell;
use std::sync::{atomic::*, Arc, Barrier};
use std::time::Duration;
use std::{array, mem, ptr, thread};

#[cfg(feature = "use_ahash")]
use ahash::RandomState;
#[cfg(feature = "use_fxhash")]
use fxhash::FxBuildHasher as RandomState;
#[cfg(not(any(feature = "use_ahash", feature = "use_fxhash")))]
use std::collections::hash_map::RandomState;

use anyhow::{Context as _, Result};
use clap::Parser;
use dashmap::DashMap;
use futures::{executor::block_on, future::join_all};
use local_ip_address::local_ip;
use rrppcc::{type_alias::*, *};

#[allow(unused_imports)]
use quanta::Instant;

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use nos::ctrl::*;
use nos::ec::EcConfig;
use nos::objstore::{ObjMeta, ObjStoreStats, RpcRes};
use nos::rpc_types::*;
use nos::utils::async_sleep;
use nos::{Args, Key};

thread_local! {
    /// RPC resources for the current thread.
    static RESOURCES: UnsafeCell<Option<RpcRes>> = const { UnsafeCell::new(None) };
}

enum Object {
    Primary(Vec<u8>),
    Replica(Vec<u8>),
}

/// A local object store.
struct ObjStore<K>
where
    K: Key,
{
    /// Cluster information. Specified in `new` by the user.
    cluster: Cluster,
    /// Fault tolerance threshold. Specified in `new` by the user.
    p: usize,
    /// Node simulated slow-down status. Allocated in `new`.
    slow_down: Vec<AtomicBool>,

    /// Real object key-value store. Allocated in `new`.
    store: DashMap<K, Object, RandomState>,
    /// Replication index.
    repl_index: DashMap<K, Vec<usize>, RandomState>,

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
    fn new(cluster: &Cluster, p: usize) -> Self {
        Self {
            cluster: cluster.clone(),
            p,
            slow_down: (0..cluster.size())
                .map(|_| AtomicBool::new(false))
                .collect::<Vec<_>>(),

            store: DashMap::with_capacity_and_hasher(
                Self::INITIAL_MAP_CAPACITY,
                RandomState::default(),
            ),
            repl_index: DashMap::with_capacity_and_hasher(
                Self::INITIAL_MAP_CAPACITY,
                RandomState::default(),
            ),

            halt: AtomicBool::new(false),
            stats: ObjStoreStats::new(),
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
    #[inline]
    fn insert(&self, key: K, primary: usize, value_addr: *const u8, len: usize) {
        // Insert the object into the key-value store
        let value = {
            let mut value = Vec::with_capacity(len);
            unsafe {
                std::ptr::copy_nonoverlapping(value_addr, value.as_mut_ptr(), len);
                value.set_len(len);
            };
            value
        };

        self.store.insert(
            key,
            if primary == self.cluster.rank() {
                Object::Primary(value)
            } else {
                Object::Replica(value)
            },
        );
    }

    /// Retrieve the given object from the object store and copy it into the
    /// given buffer for further RDMA transmission.
    #[inline]
    fn get(&self, key: K) -> Option<Vec<u8>> {
        let entry = self.store.get(&key)?;
        let val = match entry.value() {
            Object::Primary(value) => value,
            Object::Replica(value) => value,
        };
        Some(val.clone())
    }

    /// Request handler callback.
    #[inline]
    pub async fn rpc_handler(&self, req: RequestHandle) -> MsgBuf {
        let rpc = req.rpc();

        // Check request sanity.
        debug_assert!(
            (BEGIN_DATA_RPC..=END_DATA_RPC).contains(&req.req_type()),
            "RPC type {} incorrectly handled by `rpc_handler`",
            req.req_type()
        );
        debug_assert!(
            req.req_type() != RPC_DATA_SPLIT_READ,
            "data split request sent to non EC-Split baseline"
        );

        // Get request header and determine request type.
        let req_buf = req.req_buf();
        debug_assert!(
            req_buf.len() >= mem::size_of::<ObjMeta<K>>(),
            "request too short ({} bytes) to contain a full header",
            req_buf.len()
        );

        // SAFETY:
        // - Clients definitely send a header at the beginning of the request;
        // - Data validity is ensured by NIC;
        // - There is enough space for the header (checked by the assertion).
        let hdr = unsafe { <ObjMeta<K>>::from_ptr(req_buf.as_ptr()) };
        debug_assert!(
            req.req_type() != RPC_DATA_WRITE || hdr.len > 0,
            "empty write request"
        );

        let primary = (hdr.key % self.cluster.size() as u64) as usize;

        if req.req_type() == RPC_DATA_WRITE {
            log::trace!("received write request for key {}", hdr.key);

            // Simulate slow down.
            if self.slow_down[self.cluster.rank()].load(Ordering::Relaxed) {
                // Simulate slow down.
                async_sleep(Duration::from_millis(1)).await;
            }

            // let (mut t_repl, mut t_kvs) = (0u64, 0u64);
            // let start_time = Instant::now();
            // let repl_index = {
            //     let mut repl_index = Vec::with_capacity(self.p + 1);
            //     repl_index.push(primary);
            //     for i in 1..self.cluster.size() {
            //         let node_id = (primary + i) % self.cluster.size();
            //         if self.slow_down[node_id].load(Ordering::Relaxed) {
            //             continue;
            //         }
            //         repl_index.push(node_id);
            //         if repl_index.len() == self.p + 1 {
            //             break;
            //         }
            //     }
            //     repl_index
            // };
            let repl_index = (0..=self.p)
                .map(|i| (primary + i) % self.cluster.size())
                .collect::<Vec<_>>();
            self.repl_index.insert(hdr.key, repl_index.clone());

            self.insert(
                hdr.key,
                primary,
                unsafe { req_buf.as_ptr().add(<ObjMeta<K>>::len()) },
                hdr.len as usize,
            );

            // let t1 = Instant::now();
            // t_kvs += (t1 - start_time).as_nanos() as u64;

            // Perform replication if I am primary.
            if primary == self.cluster.rank() {
                // 1. Allocate message buffers.
                let mut req_bufs = (0..self.p)
                    .map(|_| rpc.alloc_msgbuf(<ObjMeta<K>>::len() + hdr.len as usize))
                    .collect::<Vec<_>>();
                let mut resp_bufs = (0..self.p)
                    .map(|_| rpc.alloc_msgbuf(mem::size_of::<u64>()))
                    .collect::<Vec<_>>();

                // It should be OK to directly copy the incoming requests.
                for nested_req_buf in req_bufs.iter_mut() {
                    unsafe {
                        ptr::copy_nonoverlapping(
                            req_buf.as_ptr(),
                            nested_req_buf.as_mut_ptr(),
                            <ObjMeta<K>>::len() + hdr.len as usize,
                        );
                    }
                }

                // 2. Prepare sessions.
                let sessions = (0..self.p)
                    .map(|i| {
                        let node_id = repl_index[i + 1];
                        debug_assert_ne!(node_id, self.cluster.rank(), "replicating to self");
                        RESOURCES.with(|res| {
                            let res = unsafe { &*res.get() }.as_ref().unwrap();
                            res.session_to(rpc, node_id)
                        })
                    })
                    .collect::<Vec<_>>();

                // 3. Make async requests.
                let mut repl_futs = Vec::with_capacity(self.p);
                let mut resp_buf_slice = &mut resp_bufs[..];
                for i in 0..self.p {
                    let sess = &sessions[i];
                    let nested_req_buf = &req_bufs[i];
                    let nested_resp_buf = {
                        let (head, tail) = resp_buf_slice.split_first_mut().unwrap();
                        resp_buf_slice = tail;
                        head
                    };

                    repl_futs.push(sess.request(RPC_DATA_WRITE, nested_req_buf, nested_resp_buf));
                }

                // 4. Concurrently wait for all replication requests to finish.
                join_all(repl_futs.into_iter()).await;
            }
            // t_repl += t1.elapsed().as_nanos() as u64;
            // self.stats.record(0, t_repl, t_kvs, 0);
        }

        // Response pointer.
        let mut resp_buf = req.pre_resp_buf();

        let resp_payload_len = if req.req_type() == RPC_DATA_READ {
            let value = self.get(hdr.key);
            if let Some(value) = value {
                if value.len() > resp_buf.capacity() - mem::size_of::<u64>() {
                    resp_buf = rpc.alloc_msgbuf(value.len() + mem::size_of::<u64>());
                }
                let resp_payload = unsafe { resp_buf.as_mut_ptr().add(mem::size_of::<u64>()) };
                unsafe { ptr::copy_nonoverlapping(value.as_ptr(), resp_payload, value.len()) };
                value.len()
            } else {
                0
            }
        } else {
            0
        };

        let retval = resp_buf.as_ptr() as *mut u64;
        unsafe { *retval = resp_payload_len as u64 };
        resp_buf.set_len(resp_payload_len + mem::size_of::<u64>());
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
                RPC_MAKE_SLOWED => self.slow_down[peer_id].store(true, Ordering::Relaxed),
                RPC_MAKE_SLOW_RECOVERED => self.slow_down[peer_id].store(false, Ordering::Relaxed),
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
    let nthreads = {
        let mut nthreads = NThreads::load_toml_file(&args.config_file)?;
        nthreads.rpc += nthreads.ec;
        nthreads.ec = 0;
        nthreads
    };
    if cluster.rank() == 0 {
        log::info!("===== Repl baseline =====");
        log::info!("Replications: {}", ec_config.p);
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
    // Initiate object store
    let objstore = Arc::new(<ObjStore<u64>>::new(&cluster, ec_config.p));

    // Initialize RPC
    let nexus = Nexus::new((
        local_ip().expect("failed to get local IPv4 address"),
        cluster.me().port(),
    ));
    log::info!("Nexus working on {:?}", nexus.uri());

    // Spawn RPC handler threads
    let mut listener_handles = Vec::with_capacity(nthreads.rpc);

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

            // Prepare RPC and wait for all RPC threads in the cluster to be ready.
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
