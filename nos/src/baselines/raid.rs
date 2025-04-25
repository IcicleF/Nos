use std::cell::UnsafeCell;
use std::cmp::{self, Reverse};
use std::sync::{atomic::*, Arc, Barrier, Mutex};
use std::{array, hash, hint, mem, ptr, slice, thread};

#[cfg(feature = "use_ahash")]
use ahash::RandomState;
#[cfg(feature = "use_fxhash")]
use fxhash::FxBuildHasher as RandomState;
#[cfg(not(any(feature = "use_ahash", feature = "use_fxhash")))]
use std::collections::hash_map::RandomState;

use anyhow::{Context as _, Result};
use clap::Parser;
use crossbeam::queue::ArrayQueue;
use dashmap::DashMap;
use futures::{executor::block_on, future::join_all};
use local_ip_address::local_ip;
use priority_queue::PriorityQueue;
use rrppcc::{type_alias::*, *};

#[allow(unused_imports)]
use quanta::Instant;

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use nos::ctrl::*;
use nos::ec::{isal_wrappings as isal, EcConfig};
use nos::objstore::{ObjMeta, ObjStoreStats, RpcRes, MAX_VALUE_SIZE};
use nos::rpc_types::*;
use nos::{Args, Key};

trait RaidKey: Key {
    /// Return the first key in the key's RAID group.
    fn raid_group(&self, k: usize) -> Self;

    /// Return the primary node of this key.
    fn raid_primary(&self, n: usize) -> usize;
}

impl<K> RaidKey for K
where
    K: Key,
{
    #[inline]
    fn raid_group(&self, k: usize) -> Self {
        self.div(k as u64).mul(k as u64)
    }

    /// Equivalent to normal modulo operation.
    #[inline]
    fn raid_primary(&self, n: usize) -> usize {
        self.rem(n as u64) as usize
    }
}

type Xid = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
struct RaidRpcRequest<K>
where
    K: Key,
{
    /// Object information.
    meta: ObjMeta<K>,

    /// The XID of current operation.
    xid: Xid,

    /// Latest committed XID.
    committed: Xid,
}

impl<K> RaidRpcRequest<K>
where
    K: Key,
{
    #[inline]
    fn new(meta: ObjMeta<K>, xid: Xid, committed: Xid) -> Self {
        Self {
            meta,
            xid,
            committed,
        }
    }

    #[inline]
    fn len() -> usize {
        mem::size_of::<Self>()
    }

    #[inline]
    unsafe fn from_ptr<'a>(pointer: *const u8) -> &'a Self {
        #[inline(never)]
        #[cold]
        fn alignment_check_fail(pointer: *const u8, alignment: usize) {
            panic!(
                "object metadata pointer {:p} is not aligned (align = {})",
                pointer, alignment
            );
        }

        if pointer as usize % mem::align_of::<Self>() != 0 {
            alignment_check_fail(pointer, mem::align_of::<Self>());
        }
        &*(pointer as *const Self)
    }
}

/// An update in the waiting queue pending installtion caused by XID updates.
struct PendingKvEntry<K>(Xid, ObjMeta<K>, Vec<u8>)
where
    K: Key;

impl<K> hash::Hash for PendingKvEntry<K>
where
    K: Key,
{
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        (self.0, self.1.key).hash(state);
    }
}

impl<K> PartialEq for PendingKvEntry<K>
where
    K: Key,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1.key == other.1.key
    }
}

impl<K> Eq for PendingKvEntry<K> where K: Key {}

thread_local! {
    /// RPC resources for the current thread.
    static RESOURCES: UnsafeCell<Option<RpcRes>> = const { UnsafeCell::new(None) };
}

/// A local object store.
struct ObjStore<K>
where
    K: Key,
{
    /// Cluster information. Specified in `new` by the user.
    cluster: Cluster,

    /// EC config. Specified in `new` by the user.
    ec_config: EcConfig,

    /// Real object key-value store. Allocated in `new`.
    store: DashMap<K, Vec<u8>, RandomState>,

    /// Flag for halting the object store's listener threads.
    halt: AtomicBool,

    /// Waiting queues.
    wqs: Vec<ArrayQueue<PendingKvEntry<K>>>,

    /// Ordered waiting queues.
    #[allow(clippy::type_complexity)]
    pwqs: Vec<Mutex<PriorityQueue<PendingKvEntry<K>, Reverse<Xid>, RandomState>>>,

    /// XID.
    xid: AtomicU64,

    /// My committed XID.
    committed_xid: AtomicU64,

    /// Committed XIDs of other nodes.
    peer_committed_xids: Vec<Mutex<u64>>,

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

    /// Receive queue length.
    pub const QUEUE_LEN: usize = 1usize << 16;

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

            wqs: (0..cluster.size())
                .map(|_| ArrayQueue::new(Self::QUEUE_LEN))
                .collect(),
            pwqs: (0..cluster.size())
                .map(|_| {
                    Mutex::new(PriorityQueue::with_capacity_and_hasher(
                        Self::QUEUE_LEN,
                        RandomState::default(),
                    ))
                })
                .collect(),
            xid: AtomicU64::new(1),
            committed_xid: AtomicU64::new(0),
            peer_committed_xids: (0..cluster.size()).map(|_| Mutex::new(0)).collect(),
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
    fn insert(&self, key: K, value_addr: *const u8, len: usize) {
        let primary = key.raid_primary(self.cluster.size());
        debug_assert_eq!(
            primary,
            self.cluster.rank(),
            "trying to insert key {} on node {} which is not the primary {}",
            key,
            self.cluster.rank(),
            primary
        );

        // Insert the object into the key-value store
        let value = {
            let mut value = Vec::with_capacity(len);
            unsafe {
                std::ptr::copy_nonoverlapping(value_addr, value.as_mut_ptr(), len);
                value.set_len(len);
            };
            value
        };

        self.store.insert(key, value);
    }

    /// Update the given key's parity in the object store.
    /// This method will automatically convert the passed-in key into the first key
    /// in its stripe.
    #[inline]
    fn update(&self, key: K, value_addr: *const u8, len: usize) {
        // Represent the stripe with the first key in it
        let key = key.raid_group(self.ec_config.k);
        let value = match self.store.get_mut(&key) {
            None => {
                let mut value = Vec::with_capacity(len);
                unsafe {
                    ptr::copy_nonoverlapping(value_addr, value.as_mut_ptr(), len);
                    value.set_len(len);
                }
                value
            }
            Some(mut prev) => {
                let prev_val = prev.value_mut();
                if prev_val.len() < len {
                    prev_val.resize(len, 0);
                }

                let max_len = cmp::max(prev_val.len(), len);
                let mut value = <Vec<u8>>::with_capacity(max_len);
                let cur = unsafe { slice::from_raw_parts(value_addr, len) };
                isal::xor_compute(value.spare_capacity_mut(), &[&prev_val[..len], cur]);

                // Copy the rest of the original parity
                if prev_val.len() > len {
                    unsafe {
                        ptr::copy_nonoverlapping(
                            prev_val.as_ptr().add(len),
                            value.as_mut_ptr().add(len) as *mut _,
                            prev_val.len() - len,
                        )
                    };
                }

                unsafe { value.set_len(max_len) };
                value
            }
        };

        // Insert without holding reference into `self.store`
        self.store.insert(key, value);
    }

    /// Retrieve the given object from the object store.
    /// If not found, return an empty byte array.
    #[inline]
    fn get(&self, key: K) -> Option<Vec<u8>> {
        self.store.get(&key).map(|v| v.value().clone())
    }

    /// Retrieve the given object from the object store and copy it into the
    /// given buffer for further RDMA transmission.
    #[inline]
    unsafe fn get_to(&self, key: K, value: *mut u8) -> usize {
        let entry = self.store.get(&key);
        if let Some(entry) = entry {
            let len = entry.value().len();
            unsafe { ptr::copy_nonoverlapping(entry.value().as_ptr(), value, len) };
            len
        } else {
            0
        }
    }

    /// Recovery RPC request handler callback.
    /// In the RAID baseline, this method should always be data read.
    #[inline]
    pub async fn recovery_rpc_handler(&self, req: RequestHandle) -> MsgBuf {
        // Check request sanity
        debug_assert!(
            (BEGIN_RECOVERY_RPC..=END_RECOVERY_RPC).contains(&req.req_type()),
            "RPC type {} incorrectly handled by `recovery_rpc_handler`",
            req.req_type()
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

        // Prepare response.
        let mut resp_buf = req.pre_resp_buf();
        let retval = resp_buf.as_ptr() as *mut u64;
        if req.req_type() == RPC_PARITY_READ {
            let resp_payload = unsafe { resp_buf.as_mut_ptr().add(mem::size_of::<u64>()) };
            let resp_payload_len = unsafe { self.get_to(hdr.key, resp_payload) };

            unsafe { *retval = resp_payload_len as u64 };
            resp_buf.set_len(resp_payload_len + mem::size_of::<u64>());
        } else {
            unsafe { *retval = 0 };
            resp_buf.set_len(mem::size_of::<u64>());
        }
        resp_buf
    }

    /// Run a encoder that installs parity updates.
    pub fn encoder(&self) {
        while !self.halt.load(Ordering::Relaxed) {
            for (primary, wq) in self.wqs.iter().enumerate() {
                let mut pwq = self.pwqs[primary].lock().unwrap();
                while let Some(item) = wq.pop() {
                    let xid = item.0;
                    pwq.push(item, Reverse(xid));
                }

                let committed_xid = self.peer_committed_xids[primary].lock().unwrap();
                while !pwq.is_empty() && pwq.peek().unwrap().1 .0 <= *committed_xid {
                    let item = pwq.pop().unwrap().0;
                    self.update(item.1.key, item.2.as_ptr(), item.1.len as usize);
                }
            }
        }
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
            "data split request only works for EC-Split"
        );

        // Get request header and determine request type.
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
        debug_assert!(
            req.req_type() != RPC_DATA_WRITE || hdr.len > 0,
            "empty write request"
        );

        let primary = hdr.key.raid_primary(self.cluster.size());

        // Compute the nodes of this RAID group
        let rg = hdr.key.raid_group(self.ec_config.k);
        let rg_parity_base =
            (rg.raid_primary(self.cluster.size()) + self.ec_config.k) % self.cluster.size();

        // If this is a write and I am the primary, then it does not contain XIDs.
        // Otherwise, this request is from the primary and I need to find the XIDs.
        let mut resp_buf = req.pre_resp_buf();
        let resp_payload_len;

        if req.req_type() == RPC_DATA_WRITE {
            resp_payload_len = 0;
            if primary == self.cluster.rank() {
                log::trace!("received write request for key {}", hdr.key);
                assert!(req_buf.len() >= <ObjMeta<K>>::len() + hdr.len as usize);

                // let (mut t_coding, mut t_repl, mut t_kvs) = (0u64, 0u64, 0u64);
                // let start_time = Instant::now();

                // Compute diff
                let diff = match self.store.get(&hdr.key) {
                    Some(v) => {
                        let old_val = v.value();
                        let min_len = cmp::min(old_val.len(), hdr.len as usize);
                        let max_len = cmp::max(old_val.len(), hdr.len as usize);

                        // Do XOR only for the common part
                        let mut diff = Vec::with_capacity(max_len);

                        isal::xor_compute(
                            &mut diff.spare_capacity_mut()[..min_len],
                            &[&old_val[..min_len], unsafe {
                                &req_buf.as_slice()
                                    [<ObjMeta<K>>::len()..(min_len + <ObjMeta<K>>::len())]
                            }],
                        );

                        // Copy the rest of the value
                        if old_val.len() > min_len {
                            unsafe {
                                ptr::copy_nonoverlapping(
                                    old_val.as_ptr().add(min_len),
                                    diff.as_mut_ptr().add(min_len) as *mut _,
                                    max_len - min_len,
                                )
                            };
                        }
                        if hdr.len as usize > min_len {
                            unsafe {
                                ptr::copy_nonoverlapping(
                                    req_buf.as_ptr().add(<ObjMeta<K>>::len() + min_len),
                                    diff.as_mut_ptr().add(min_len) as *mut _,
                                    max_len - min_len,
                                )
                            };
                        }

                        unsafe { diff.set_len(hdr.len as usize) };
                        diff
                    }
                    None => {
                        let mut diff = vec![0; hdr.len as usize];
                        diff.copy_from_slice(unsafe { &req_buf.as_slice()[<ObjMeta<K>>::len()..] });
                        diff
                    }
                };

                // t_coding += start_time.elapsed().as_nanos() as u64;

                // Perform replication.
                // 1. Allocate message buffers.
                let mut req_bufs = (0..self.ec_config.p)
                    .map(|_| rpc.alloc_msgbuf(<RaidRpcRequest<K>>::len() + hdr.len as usize))
                    .collect::<Vec<_>>();
                let mut resp_bufs = (0..self.ec_config.p)
                    .map(|_| rpc.alloc_msgbuf(mem::size_of::<u64>()))
                    .collect::<Vec<_>>();

                // let t1 = Instant::now();

                // 2. Compute updates.
                // Wrap in a scope to let slices from `from_raw_parts_mut` die immediately after use.
                {
                    let mut update_bufs = (0..self.ec_config.p)
                        .map(|i| {
                            let msgbuf = &mut req_bufs[i];
                            unsafe {
                                slice::from_raw_parts_mut(
                                    msgbuf.as_mut_ptr().add(<RaidRpcRequest<K>>::len()),
                                    hdr.len as usize,
                                )
                            }
                        })
                        .collect::<Vec<_>>();
                    self.ec_config.update(
                        &diff,
                        (hdr.key % self.ec_config.k as u64) as usize,
                        &mut update_bufs,
                    );
                }

                // let t2 = Instant::now();
                // t_coding += (t2 - t1).as_nanos() as u64;

                // 3. Prepare sessions.
                let sessions = (0..self.ec_config.p)
                    .map(|i| {
                        let node_id = (rg_parity_base + i) % self.cluster.size();
                        debug_assert_ne!(node_id, self.cluster.rank(), "replicating to self");
                        RESOURCES.with(|res| {
                            let res = unsafe { &*res.get() }.as_ref().unwrap();
                            res.session_to(rpc, node_id)
                        })
                    })
                    .collect::<Vec<_>>();

                // Send updates to backup nodes.
                let xid = self.xid.fetch_add(1, Ordering::SeqCst);
                let committed = self.committed_xid.load(Ordering::Acquire);

                let mut repl_futs = Vec::with_capacity(self.ec_config.p);
                let mut resp_buf_slice = &mut resp_bufs[..];
                for i in 0..self.ec_config.p {
                    let sess = &sessions[i];
                    let nested_req_buf = &req_bufs[i];
                    let nested_resp_buf = {
                        let (head, tail) = resp_buf_slice.split_first_mut().unwrap();
                        resp_buf_slice = tail;
                        head
                    };

                    unsafe {
                        ptr::write(
                            req_bufs[i].as_ptr() as *mut RaidRpcRequest<K>,
                            RaidRpcRequest::new(hdr, xid, committed),
                        )
                    };
                    repl_futs.push(sess.request(RPC_DATA_WRITE, nested_req_buf, nested_resp_buf));
                }

                // let t3 = Instant::now();
                // t_repl += (t3 - t2).as_nanos() as u64;

                // Insert locally to hide some latency.
                self.insert(
                    hdr.key,
                    unsafe { req_buf.as_ptr().add(<ObjMeta<K>>::len()) },
                    hdr.len as usize,
                );

                // let t4 = Instant::now();
                // t_kvs += (t4 - t3).as_nanos() as u64;

                // Concurrently wait for all replication requests to finish
                join_all(repl_futs.into_iter()).await;
                log::trace!(
                    "update of primary object {} has finished replication",
                    hdr.key
                );

                // t_repl += t4.elapsed().as_nanos() as u64;

                // Update xid.
                self.committed_xid.store(xid, Ordering::Release);

                // let total_time = start_time.elapsed().as_nanos() as u64;
                // self.stats.record(
                //     t_coding,
                //     t_repl,
                //     t_kvs,
                //     total_time - t_coding - t_repl - t_kvs,
                // );
            } else {
                // Parity update request contains a `RaidRpcRequest header`
                let (xid, committed) = unsafe {
                    let req = <RaidRpcRequest<K>>::from_ptr(req_buf.as_ptr());
                    (req.xid, req.committed)
                };

                // Compute diff and put it into the waiting queue
                let diff = unsafe {
                    let mut v = Vec::with_capacity(hdr.len as usize);
                    ptr::copy_nonoverlapping(
                        req_buf.as_ptr().add(<RaidRpcRequest<K>>::len()),
                        v.as_mut_ptr(),
                        hdr.len as usize,
                    );
                    v.set_len(hdr.len as usize);
                    v
                };

                log::trace!(
                    "received parity update request for key {}: xid {}, committed {}",
                    hdr.key,
                    xid,
                    committed
                );

                // Update peer committed XID
                let mut peer_committed_xid = self.peer_committed_xids[primary].lock().unwrap();
                *peer_committed_xid = cmp::max(*peer_committed_xid, committed);

                let mut entry = PendingKvEntry(xid, hdr, diff);
                loop {
                    match self.wqs[primary].push(entry) {
                        Ok(_) => break,
                        Err(e) => entry = e,
                    }
                }
            }
        } else if req.req_type() == RPC_DATA_READ {
            resp_payload_len = if primary == self.cluster.rank() {
                // Normal read
                let resp_payload = unsafe { resp_buf.as_mut_ptr().add(mem::size_of::<u64>()) };
                let len = unsafe { self.get_to(hdr.key, resp_payload) };
                debug_assert!(
                    len <= resp_buf.capacity(),
                    "value too large: {} > {}",
                    len,
                    resp_buf.capacity()
                );
                len
            } else {
                // Degraded read
                if !self.cluster.me().is_alive() {
                    log::warn!(
                        "logically-dead node {} is handling RPC, this must be rare or something is wrong!",
                        self.cluster.rank()
                    );
                }

                let rg_first_node = rg.raid_primary(self.cluster.size());

                // Vec<(idx, node_id)>
                let alive_nodes = (0..self.ec_config.width())
                    .map(|i| (i, (rg_first_node + i) % self.cluster.size()))
                    .filter(|(_, node)| self.cluster[*node].is_alive())
                    .collect::<Vec<_>>();
                assert!(
                    alive_nodes.len() >= self.ec_config.k,
                    "not enough nodes alive"
                );

                // Fetch encodee list overheads
                if alive_nodes[0].1 != self.cluster.rank() {
                    let req_buf = rpc.alloc_msgbuf(<ObjMeta<K>>::len());
                    let mut resp_buf = rpc.alloc_msgbuf(mem::size_of::<u64>());
                    unsafe {
                        ptr::write(
                            req_buf.as_ptr() as *mut ObjMeta<K>,
                            ObjMeta::new(hdr.key, 0, 0),
                        )
                    };

                    let sess = RESOURCES.with(|res| {
                        let res = unsafe { &*res.get() }.as_ref().unwrap();
                        res.session_to(rpc, alive_nodes[0].1)
                    });
                    sess.request(RPC_ENCODEE_LIST_FETCH, &req_buf, &mut resp_buf)
                        .await;
                }

                // Construct recovery source
                // No need to shuffle since we would like to keep as many alive data chunks as possible
                let recovery_src = &alive_nodes[..self.ec_config.k];

                // Send recovery read requests
                let futs = recovery_src
                    .iter()
                    .map(|(i, node)| async {
                        // The key to read is the real object key for data, and RAID group base for parity
                        let key = if (*i) < self.ec_config.k {
                            rg + (*i) as u64
                        } else {
                            rg
                        };

                        if *node == self.cluster.rank() {
                            self.get(key)
                        } else {
                            let req_buf = rpc.alloc_msgbuf(<ObjMeta<K>>::len());
                            let mut resp_buf =
                                rpc.alloc_msgbuf(mem::size_of::<u64>() + MAX_VALUE_SIZE);
                            unsafe {
                                ptr::write(
                                    req_buf.as_ptr() as *mut ObjMeta<K>,
                                    ObjMeta::new(key, 0, 0),
                                )
                            };

                            let sess = RESOURCES.with(|res| {
                                let res = unsafe { &*res.get() }.as_ref().unwrap();
                                res.session_to(rpc, *node)
                            });
                            sess.request(RPC_PARITY_READ, &req_buf, &mut resp_buf).await;

                            unsafe {
                                let len = *(resp_buf.as_ptr() as *const u64) as usize;
                                if len > 0 {
                                    let mut value = Vec::with_capacity(len);
                                    ptr::copy_nonoverlapping(
                                        resp_buf.as_ptr().add(mem::size_of::<u64>()),
                                        value.as_mut_ptr(),
                                        len,
                                    );
                                    value.set_len(len);
                                    Some(value)
                                } else {
                                    None
                                }
                            }
                        }
                    })
                    .collect::<Vec<_>>();

                let chunks = join_all(futs).await;

                // The value of this `if` block goes to `resp_payload_len`
                if chunks.iter().all(|c| c.is_none()) {
                    0
                } else {
                    let len = chunks
                        .iter()
                        .filter_map(|c| c.as_ref().map(|c| c.len()))
                        .max()
                        .unwrap();
                    let chunks = chunks
                        .into_iter()
                        .map(|c| c.unwrap_or(vec![0; len]))
                        .collect::<Vec<_>>();

                    // Arrange the chunks in our desired format
                    let mut data = Vec::with_capacity(self.ec_config.k);
                    let mut parity = Vec::with_capacity(self.ec_config.p);

                    let mut expected_next = 0;
                    for (i, chunk) in chunks.into_iter().enumerate() {
                        let (idx, _) = recovery_src[i];
                        if idx < self.ec_config.k {
                            while expected_next < idx {
                                data.push(vec![0; len]);
                                expected_next += 1;
                            }
                            data.push(chunk);
                            expected_next += 1;
                        } else {
                            parity.push(chunk);
                        }
                    }

                    debug_assert!(
                        data.len() <= self.ec_config.k,
                        "too many data chunks: {} vs {}",
                        data.len(),
                        self.ec_config.k
                    );
                    while data.len() < self.ec_config.k {
                        data.push(vec![0; len]);
                    }

                    // Do recovery. Need to make mutable refs die as soon as possible.
                    {
                        let mut data_ref =
                            data.iter_mut().map(|b| b.as_mut()).collect::<Vec<&mut _>>();
                        let parity_ref = parity.iter().map(|b| b.as_ref()).collect::<Vec<_>>();
                        let indices = recovery_src.iter().map(|(i, _)| *i).collect::<Vec<_>>();
                        self.ec_config.recover(&mut data_ref, &parity_ref, &indices);
                    }

                    // Now all lost chunks should be recovered. I just find the one I want.
                    let my_idx = (0..self.ec_config.k)
                        .find(|&i| (rg + i as u64) == hdr.key)
                        .unwrap();
                    unsafe {
                        let resp_payload = resp_buf.as_mut_ptr().add(mem::size_of::<u64>());
                        ptr::copy_nonoverlapping(data[my_idx].as_ptr(), resp_payload, len);
                    }
                    len
                }
            };
        } else {
            // SAFETY: all possible RPC types (read & write) are handled
            unsafe { hint::unreachable_unchecked() };
        }

        // Response pointers
        let retval = resp_buf.as_ptr() as *mut u64;
        unsafe {
            *retval = resp_payload_len as u64;
        }

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
        log::info!("===== RAID baseline =====");
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
    // Initiate object store
    let objstore = Arc::new(<ObjStore<u64>>::new(&cluster, &ec_config));

    // Initialize RPC
    let nexus = Nexus::new((
        local_ip().expect("failed to get local IPv4 address"),
        cluster.me().port(),
    ));
    log::info!("Nexus working on {:?}", nexus.uri());

    // Spawn RPC handler threads
    let mut listener_handles = Vec::with_capacity(nthreads.rpc + nthreads.ec);

    for _ in 0..nthreads.ec {
        let core_id = core_id_iter.next().unwrap();
        let objstore = objstore.clone();
        listener_handles.push(thread::spawn(move || {
            // Bind thread to core
            core_affinity::set_for_current(core_id);
            objstore.encoder()
        }));
    }

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
            // Bind core.
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
                        ty if (BEGIN_RECOVERY_RPC..=END_RECOVERY_RPC).contains(&ty) => {
                            rpc.set_handler(ty, move |req| {
                                let objstore = objstore.clone();
                                async move { objstore.recovery_rpc_handler(req).await }
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
                        _ => unreachable!(),
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
