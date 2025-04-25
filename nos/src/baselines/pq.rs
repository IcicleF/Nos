use std::cell::UnsafeCell;
use std::sync::{atomic::*, Arc, Barrier, Mutex};
use std::time::Duration;
use std::{array, cmp, hint, mem, ptr, slice, thread};

#[cfg(feature = "use_ahash")]
use ahash::RandomState;
#[cfg(feature = "use_fxhash")]
use fxhash::FxBuildHasher as RandomState;
#[cfg(not(any(feature = "use_ahash", feature = "use_fxhash")))]
use std::collections::hash_map::RandomState;

use anyhow::{Context as _, Result};
use clap::Parser;
use crossbeam::queue::SegQueue;
use dashmap::DashMap;
use futures::{executor::block_on, future::join_all};
use local_ip_address::local_ip;
use rrppcc::{type_alias::*, *};

#[allow(unused_imports)]
use quanta::Instant;

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use nos::ctrl::*;
use nos::ec::{isal_wrappings as isal, EcConfig};
use nos::objstore::{ObjMeta, ObjStoreStats, RpcRes, MAX_VALUE_SIZE};
use nos::rpc_types::*;
use nos::utils::async_sleep;
use nos::{Args, Key};

#[allow(dead_code)]
mod avltree {
    use std::cmp::Ordering;
    use std::ptr::{self, NonNull};

    type Value = super::ecalloc::EcAllocHeader;

    pub struct AvlNode {
        pub item: *mut Value,
        left: *mut AvlNode,
        right: *mut AvlNode,
        parent: *mut AvlNode,
        pub next: *mut AvlNode,
        pub prev: *mut AvlNode,
        height: i64,
    }

    impl AvlNode {
        fn new(value: *mut Value, parent: *mut AvlNode) -> Self {
            Self {
                item: value,
                left: ptr::null_mut(),
                right: ptr::null_mut(),
                parent,
                next: ptr::null_mut(),
                prev: ptr::null_mut(),
                height: 1,
            }
        }

        fn update_height(&mut self) {
            let lheight = if let Some(mut left) = NonNull::new(self.left) {
                unsafe { left.as_mut().height }
            } else {
                0
            };
            let rheight = if let Some(mut right) = NonNull::new(self.right) {
                unsafe { right.as_mut().height }
            } else {
                0
            };
            self.height = 1 + lheight.max(rheight);
        }

        fn balance(&self) -> i64 {
            let lheight = if let Some(mut left) = NonNull::new(self.left) {
                unsafe { left.as_mut().height }
            } else {
                0
            };
            let rheight = if let Some(mut right) = NonNull::new(self.right) {
                unsafe { right.as_mut().height }
            } else {
                0
            };
            lheight - rheight
        }

        unsafe fn lrotate(nodep: *mut *mut Self) {
            let node = *nodep;
            let parent = (*node).parent;
            let right = (*node).right;

            *nodep = right;
            (*right).parent = parent;
            (*node).right = (*right).left;
            if !(*right).left.is_null() {
                (*(*right).left).parent = node;
            }
            (*right).left = node;
            (*node).parent = right;

            (*node).update_height();
            (*right).update_height();
            if !parent.is_null() {
                (*parent).update_height();
            }
        }

        unsafe fn rrotate(nodep: *mut *mut Self) {
            let node = *nodep;
            let parent = (*node).parent;
            let left = (*node).left;

            *nodep = left;
            (*left).parent = parent;
            (*node).left = (*left).right;
            if !(*left).right.is_null() {
                (*(*left).right).parent = node;
            }
            (*left).right = node;
            (*node).parent = left;

            (*node).update_height();
            (*left).update_height();
            if !parent.is_null() {
                (*parent).update_height();
            }
        }
    }

    pub struct AvlTree {
        root: *mut AvlNode,
        head: *mut AvlNode,
        tail: *mut AvlNode,
        item_compare: fn(*const Value, *const Value) -> Ordering,
    }

    impl AvlTree {
        pub fn new(item_compare: fn(*const Value, *const Value) -> Ordering) -> Self {
            Self {
                root: ptr::null_mut(),
                head: ptr::null_mut(),
                tail: ptr::null_mut(),
                item_compare,
            }
        }

        fn do_rotation(&mut self, mut parent: *mut AvlNode) {
            while !parent.is_null() {
                let node = unsafe { &mut *parent };
                parent = node.parent;
                let left = node.left;
                let right = node.right;

                let balance = node.balance();
                if balance < -1 {
                    let son_balance = unsafe { (*right).balance() };
                    if son_balance <= 0 {
                        // RR
                        unsafe {
                            if parent.is_null() {
                                AvlNode::lrotate(&mut self.root);
                            } else {
                                AvlNode::lrotate(if (node as *mut _) == (*parent).left {
                                    &mut (*parent).left
                                } else {
                                    &mut (*parent).right
                                });
                            }
                        }
                        continue;
                    }
                    if son_balance > 0 {
                        // RL
                        unsafe {
                            AvlNode::rrotate(&mut node.right);
                            if parent.is_null() {
                                AvlNode::lrotate(&mut self.root);
                            } else {
                                AvlNode::lrotate(if (node as *mut _) == (*parent).left {
                                    &mut (*parent).left
                                } else {
                                    &mut (*parent).right
                                });
                            }
                        }
                        continue;
                    }
                    unreachable!();
                } else if balance > 1 {
                    let son_balance = unsafe { (*left).balance() };
                    if son_balance >= 0 {
                        // LL
                        unsafe {
                            if parent.is_null() {
                                AvlNode::rrotate(&mut self.root);
                            } else {
                                AvlNode::rrotate(if (node as *mut _) == (*parent).left {
                                    &mut (*parent).left
                                } else {
                                    &mut (*parent).right
                                });
                            }
                        }
                        continue;
                    }
                    if son_balance < 0 {
                        // LR
                        unsafe {
                            AvlNode::lrotate(&mut node.left);
                            if parent.is_null() {
                                AvlNode::rrotate(&mut self.root);
                            } else {
                                AvlNode::rrotate(if (node as *mut _) == (*parent).left {
                                    &mut (*parent).left
                                } else {
                                    &mut (*parent).right
                                });
                            }
                        }
                        continue;
                    }
                    unreachable!();
                } else {
                    node.update_height();
                }
            }
        }

        pub fn insert(&mut self, item: *mut Value) -> NonNull<AvlNode> {
            let mut parent = ptr::null_mut();
            let mut nodep: *mut *mut AvlNode = &mut self.root;

            while let Some(mut node) = unsafe { NonNull::new(*nodep) } {
                let node = unsafe { node.as_mut() };
                match (self.item_compare)(item, node.item) {
                    Ordering::Less => {
                        parent = node;
                        nodep = &mut node.left;
                    }
                    Ordering::Greater => {
                        parent = node;
                        nodep = &mut node.right;
                    }
                    Ordering::Equal => return NonNull::from(node),
                };
            }

            let node = {
                let node = Box::new(AvlNode::new(item, parent));
                Box::leak(node)
            };
            unsafe { *nodep = node };

            if parent.is_null() {
                self.head = node;
                self.tail = node;
            } else {
                let parent = unsafe { &mut *parent };
                if parent.left == node {
                    let prev = parent.prev;
                    node.prev = prev;
                    node.next = parent;
                    parent.prev = node;
                    if let Some(mut prev) = NonNull::new(prev) {
                        unsafe { prev.as_mut().next = node };
                    } else {
                        self.head = node;
                    }
                } else {
                    let next = parent.next;
                    node.next = next;
                    node.prev = parent;
                    parent.next = node;
                    if let Some(mut next) = NonNull::new(next) {
                        unsafe { next.as_mut().prev = node };
                    } else {
                        self.tail = node;
                    }
                }
            }

            self.do_rotation(parent);
            NonNull::new(node).unwrap()
        }

        pub fn remove_node(&mut self, mut node: *mut AvlNode) {
            let parent = ptr::null_mut();
            loop {
                let replace = {
                    let node = unsafe { &mut *node };
                    if node.left.is_null() {
                        // Modify tree structure.
                        let parent = node.parent;
                        let right = node.right;
                        if parent.is_null() {
                            self.root = right;
                        } else {
                            let parent = unsafe { &mut *parent };
                            if parent.left == node {
                                parent.left = right;
                            } else {
                                parent.right = right;
                            }
                        }
                        if !right.is_null() {
                            unsafe { (*right).parent = parent };
                        }

                        // Modify double linked list.
                        let prev = node.prev;
                        let next = node.next;
                        if !next.is_null() {
                            unsafe { (*next).prev = prev };
                        } else {
                            self.tail = prev;
                        }
                        if !prev.is_null() {
                            unsafe { (*prev).next = next };
                        } else {
                            self.head = next;
                        }
                        break;
                    }
                    if node.right.is_null() {
                        // Modify tree structure.
                        let parent = node.parent;
                        let left = node.left;
                        if parent.is_null() {
                            self.root = left;
                        } else {
                            let parent = unsafe { &mut *parent };
                            if parent.left == node {
                                parent.left = left;
                            } else {
                                parent.right = left;
                            }
                        }
                        if !left.is_null() {
                            unsafe { (*left).parent = parent };
                        }

                        // Modify double linked list.
                        let prev = node.prev;
                        let next = node.next;
                        if !next.is_null() {
                            unsafe { (*next).prev = prev };
                        } else {
                            self.tail = prev;
                        }
                        if !prev.is_null() {
                            unsafe { (*prev).next = next };
                        } else {
                            self.head = next;
                        }
                        break;
                    }

                    let replace = unsafe { &mut *node.prev };
                    node.item = replace.item;
                    replace
                };

                unsafe { (*replace.item).node = node };
                node = replace;
            }

            self.do_rotation(parent);
            let _ = unsafe { Box::from_raw(node) };
        }

        pub fn remove(&mut self, item: *const Value) -> *mut Value {
            let node = self.search(item);
            if node.is_null() {
                return ptr::null_mut();
            }
            let ret = unsafe { (*node).item };
            self.remove_node(node);
            ret
        }

        pub fn search(&self, item: *const Value) -> *mut AvlNode {
            let mut node = self.root;
            while !node.is_null() {
                node = {
                    let node = unsafe { &*node };
                    match (self.item_compare)(item, node.item) {
                        Ordering::Less => node.left,
                        Ordering::Greater => node.right,
                        Ordering::Equal => break,
                    }
                };
            }
            node
        }

        pub fn search_close(&self, item: *const Value) -> *mut AvlNode {
            let mut node = self.root;
            while !node.is_null() {
                node = {
                    let node = unsafe { &*node };
                    match (self.item_compare)(item, node.item) {
                        Ordering::Less => {
                            if !node.left.is_null() {
                                node.left
                            } else {
                                break;
                            }
                        }
                        Ordering::Greater => {
                            if !node.right.is_null() {
                                node.right
                            } else {
                                break;
                            }
                        }
                        Ordering::Equal => break,
                    }
                };
            }
            node
        }
    }
}

#[allow(dead_code)]
mod ecalloc {
    use super::avltree::*;
    use std::{cmp, ptr, sync::Mutex};

    const USING: u64 = 0x1;

    #[derive(Clone, Copy)]
    pub struct EcAllocHeader {
        addr: usize,
        size: usize,
        flag: u64,

        next: *mut EcAllocHeader,
        prev: *mut EcAllocHeader,
        pub node: *mut AvlNode,
    }

    impl Default for EcAllocHeader {
        fn default() -> Self {
            Self {
                addr: 0,
                size: 0,
                flag: 0,
                next: ptr::null_mut(),
                prev: ptr::null_mut(),
                node: ptr::null_mut(),
            }
        }
    }

    struct EcAllocInner {
        free_tree: AvlTree,
        used_tree: AvlTree,
        size: usize,
        used: usize,
    }

    fn addr_compare(a: *const EcAllocHeader, b: *const EcAllocHeader) -> cmp::Ordering {
        let left = unsafe { (*a).addr };
        let right = unsafe { (*b).addr };
        left.cmp(&right)
    }

    fn size_compare(a: *const EcAllocHeader, b: *const EcAllocHeader) -> cmp::Ordering {
        let left = unsafe { (*a).size };
        let right = unsafe { (*b).size };
        left.cmp(&right)
    }

    pub struct EcAlloc(Mutex<EcAllocInner>);

    impl EcAlloc {
        pub fn new(size: usize) -> Self {
            let mut free_tree = AvlTree::new(size_compare);
            let used_tree = AvlTree::new(addr_compare);

            let header = {
                let header = Box::new(EcAllocHeader {
                    addr: 0,
                    size,
                    flag: 0,
                    next: ptr::null_mut(),
                    prev: ptr::null_mut(),
                    node: ptr::null_mut(),
                });
                Box::leak(header)
            };
            header.node = free_tree.insert(header).as_ptr();

            Self(Mutex::new(EcAllocInner {
                free_tree,
                used_tree,
                size,
                used: 0,
            }))
        }

        pub fn alloc(&self, size: usize) -> Result<usize, ()> {
            let size = size.next_multiple_of(16);

            let mut tmp = EcAllocHeader::default();
            tmp.size = size;

            let mut ecalloc = self.0.lock().unwrap();
            let node = ecalloc.free_tree.search_close(&tmp as *const _);
            if node.is_null() {
                // No memory.
                return Err(());
            }

            let mut node = unsafe { &mut *node };
            let mut header = {
                let header = node.item;
                assert!(!header.is_null());
                unsafe { &mut *header }
            };
            if header.size < size && !node.next.is_null() {
                node = unsafe { &mut *node.next };
                header = unsafe { &mut *node.item };
            }
            if header.size < size {
                // No memory.
                return Err(());
            }

            let ret = header.addr;
            ecalloc.used += size;

            // Split the node into used and free.
            ecalloc.free_tree.remove_node(node);
            if header.size > size {
                let newer = {
                    let newer = Box::new(EcAllocHeader {
                        addr: header.addr,
                        size: size,
                        flag: USING,
                        next: header,
                        prev: header.prev,
                        node: ptr::null_mut(),
                    });
                    Box::leak(newer)
                };
                newer.node = ecalloc.used_tree.insert(newer).as_ptr();

                header.prev = newer;
                header.addr += size;
                header.size -= size;
                header.node = ecalloc.free_tree.insert(header).as_ptr();
            } else {
                header.flag |= USING;
                header.node = ecalloc.used_tree.insert(header).as_ptr();
            }

            Ok(ret)
        }

        pub fn check(&self, addr: usize, _: usize) -> i32 {
            let ecalloc = self.0.lock().unwrap();

            let mut tmp = EcAllocHeader::default();
            tmp.addr = addr;

            let node = ecalloc.used_tree.search(&tmp);
            if node.is_null() {
                return -1;
            }
            let node = unsafe { &*node };
            let header = node.item;

            if header.is_null() {
                return -1;
            } else {
                let header = unsafe { &*header };
                if header.flag & USING == 0 {
                    return -2;
                }
            }
            0
        }

        pub fn free(&self, addr: usize) {
            let mut ecalloc = self.0.lock().unwrap();

            let mut tmp = EcAllocHeader::default();
            tmp.addr = addr;

            let header = ecalloc.used_tree.remove(&tmp as *const _);
            assert!(
                !header.is_null(),
                "freeing wrong addr {:p}",
                addr as *const u8
            );
            let header = unsafe { &mut *header };
            ecalloc.used -= header.size;
            header.flag &= !USING;

            // Combine neighbor behind.
            loop {
                let next = header.next;
                if next.is_null() {
                    break;
                }
                let next = unsafe { &mut *next };
                if next.flag & USING != 0 {
                    break;
                }
                assert!(next.addr == header.addr + header.size);

                // Combine the two.
                header.size += next.size;
                header.next = next.next;
                if !header.next.is_null() {
                    unsafe { (*header.next).prev = header };
                }

                // Clean the next.
                ecalloc.free_tree.remove_node(next.node);
                let _ = unsafe { Box::from_raw(next) };
            }

            // Combine neighbor front.
            loop {
                let prev = header.prev;
                if prev.is_null() {
                    break;
                }
                let prev = unsafe { &mut *prev };
                if prev.flag & USING != 0 {
                    break;
                }
                assert!(header.addr == prev.addr + prev.size);

                // Combine the two.
                header.size += prev.size;
                header.addr = prev.addr;
                header.prev = prev.prev;
                if !header.prev.is_null() {
                    unsafe { (*header.prev).next = header };
                }

                // Clean the prev.
                ecalloc.free_tree.remove_node(prev.node);
                let _ = unsafe { Box::from_raw(prev) };
            }

            header.node = ecalloc.free_tree.insert(header).as_ptr();
        }
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

    /// Storage offset of this object.
    /// Valid only for primary -> backup writes.
    offset: usize,
}

impl<K> RaidRpcRequest<K>
where
    K: Key,
{
    #[inline]
    fn new(meta: ObjMeta<K>, xid: Xid, committed: Xid, offset: usize) -> Self {
        Self {
            meta,
            xid,
            committed,
            offset,
        }
    }

    #[inline]
    fn len() -> usize {
        mem::size_of::<Self>()
    }
}

/// An update in the waiting queue pending installtion caused by XID updates.
struct PendingKvEntry<K>(RaidRpcRequest<K>, Vec<u8>)
where
    K: Key;

thread_local! {
    /// RPC resources for the current thread.
    static RESOURCES: UnsafeCell<Option<RpcRes>> = const { UnsafeCell::new(None) };
}

const ALLOC_UNIT: usize = 4 << 10;

/// A local object store.
struct ObjStore<K>
where
    K: Key,
{
    /// Cluster information. Specified in `new` by the user.
    cluster: Cluster,
    /// EC config. Specified in `new` by the user.
    ec_config: EcConfig,
    /// Node simulated slow-down status. Allocated in `new`.
    slow_down: Vec<AtomicBool>,

    /// Object index. Allocated in `new`.
    index: DashMap<K, (usize, usize), RandomState>,
    /// Primary buffer. Allocated in `new`.
    #[allow(dead_code)]
    primarybuf: DashMap<K, Vec<u8>, RandomState>,
    /// Value buffer. Allocated in `new`.
    valuebuf: Vec<DashMap<usize, [u8; ALLOC_UNIT], RandomState>>,
    /// Value buffer allocator. Allocated in `new`.
    alloc: Vec<ecalloc::EcAlloc>,

    /// Waiting queues.
    wqs: Vec<SegQueue<PendingKvEntry<K>>>,
    /// XID.
    xid: AtomicU64,
    /// My committed XID.
    committed_xid: AtomicU64,
    /// Committed XIDs of other nodes.
    peer_committed_xids: Vec<Mutex<u64>>,

    /// Flag for halting the object store's listener threads.
    halt: AtomicBool,
    /// Performance statistics, dead when normal benchmarking.
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

    /// Virtual buffer size in each group.
    pub const VIRTUAL_BUFFER_SIZE: usize = 64usize << 30;

    /// Create a RAID baseline object store instance.
    fn new(cluster: &Cluster, ec_config: &EcConfig) -> Self {
        Self {
            cluster: cluster.clone(),
            ec_config: ec_config.clone(),
            slow_down: (0..cluster.size())
                .map(|_| AtomicBool::new(false))
                .collect::<Vec<_>>(),

            index: DashMap::with_capacity_and_hasher(
                Self::INITIAL_MAP_CAPACITY,
                RandomState::default(),
            ),
            primarybuf: DashMap::with_capacity_and_hasher(
                Self::INITIAL_MAP_CAPACITY,
                RandomState::default(),
            ),
            valuebuf: (0..cluster.size())
                .map(|_| {
                    DashMap::with_capacity_and_hasher(
                        Self::INITIAL_MAP_CAPACITY,
                        RandomState::default(),
                    )
                })
                .collect(),
            alloc: (0..cluster.size())
                .map(|_| ecalloc::EcAlloc::new(Self::VIRTUAL_BUFFER_SIZE))
                .collect(),

            wqs: (0..cluster.size()).map(|_| SegQueue::new()).collect(),
            xid: AtomicU64::new(1),
            committed_xid: AtomicU64::new(0),
            peer_committed_xids: (0..cluster.size()).map(|_| Mutex::new(0)).collect(),

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

    /// Get the RAID group (RG, can be viewed as PG) ID of the given key.
    #[inline(always)]
    fn raid_group(&self, key: K) -> usize {
        let rem = (key % (self.ec_config.k * self.cluster.size()) as u64) as usize;
        rem / self.ec_config.k
    }

    /// Insert the given object into the object store.
    /// This node must be the primary node of the object.
    #[inline]
    fn insert(&self, key: K, value_addr: *const u8, len: usize) -> usize {
        let primary = (key % self.cluster.size() as u64) as usize;
        debug_assert_eq!(
            primary,
            self.cluster.rank(),
            "trying to insert key {} on node {} which is not the primary {}",
            key,
            self.cluster.rank(),
            primary
        );

        // Insert into primary value store.
        let mut primary_buf = self.primarybuf.entry(key).or_insert_with(|| vec![0; len]);
        unsafe {
            ptr::copy_nonoverlapping(value_addr, primary_buf.as_mut_ptr(), len);
        }

        // Allocate the object in the virtual buffer.
        let rg = self.raid_group(key);
        let offset = self.alloc[rg].alloc(len).expect("no memory");
        let first_byte_in = offset / ALLOC_UNIT;
        let last_byte_in = (offset + len - 1) / ALLOC_UNIT;

        // Realize the buffers and copy the value into them.
        let mut copied = 0;
        for i in first_byte_in..=last_byte_in {
            let mut value = self.valuebuf[rg]
                .entry(i)
                .or_insert_with(|| [0; ALLOC_UNIT]);

            let base = i * ALLOC_UNIT;
            let start = base.max(offset) - base;
            let end = (base + ALLOC_UNIT).min(offset + len) - base;
            let value = &mut value.as_mut()[start..end];
            unsafe {
                ptr::copy_nonoverlapping(value_addr.add(copied), value.as_mut_ptr(), end - start)
            };
            copied += end - start;
        }
        assert!(copied == len);
        offset
    }

    /// Update the given key's parity in the object store.
    /// This method will automatically convert the passed-in key into the first key
    /// in its stripe.
    #[inline]
    fn update(&self, rg: usize, value_addr: *const u8, offset: usize, len: usize) {
        let first_byte_in = offset / ALLOC_UNIT;
        let last_byte_in = (offset + len - 1) / ALLOC_UNIT;

        // Realize the buffers and copy the value into them.
        let mut updated = 0;
        for i in first_byte_in..=last_byte_in {
            let mut value = self.valuebuf[rg]
                .entry(i)
                .or_insert_with(|| [0; ALLOC_UNIT]);

            let base = i * ALLOC_UNIT;
            let start = base.max(offset) - base;
            let end = (base + ALLOC_UNIT).min(offset + len) - base;

            let value = &mut value.as_mut()[start..end];
            isal::xor_update(value, unsafe {
                slice::from_raw_parts(value_addr.add(updated), end - start)
            });

            updated += end - start;
        }
        assert!(updated == len);
    }

    /// Retrieve the given object from the object store.
    /// If not found, return an empty byte array.
    #[inline]
    fn get(&self, key: K) -> Option<Vec<u8>> {
        let (offset, len) = *self.index.get(&key)?.value();

        let rg = self.raid_group(key);
        let mut res = vec![0u8; len];
        let first_byte_in = offset / ALLOC_UNIT;
        let last_byte_in = (offset + len - 1) / ALLOC_UNIT;

        // Copy the value into the response buffer.
        let mut copied = 0;
        for i in first_byte_in..=last_byte_in {
            let value = self.valuebuf[rg]
                .entry(i)
                .or_insert_with(|| [0; ALLOC_UNIT]);

            let base = i * ALLOC_UNIT;
            let start = base.max(offset) - base;
            let end = (base + ALLOC_UNIT).min(offset + len) - base;

            let value = &value.as_ref()[start..end];
            unsafe {
                ptr::copy_nonoverlapping(value.as_ptr(), res.as_mut_ptr().add(copied), end - start)
            };
            copied += end - start;
        }
        assert!(copied == len);
        Some(res)
    }

    /// Retrieve the given object from the object store and copy it into the
    /// given buffer for further RDMA transmission.
    #[inline]
    unsafe fn get_to(&self, key: K, dst: *mut u8) -> usize {
        let primary_len = self
            .primarybuf
            .get(&key)
            .map_or(0, |e| std::hint::black_box(e.value().len()));
        let (offset, len) = match self.index.get(&key) {
            Some(e) => {
                assert!(e.value().1 == primary_len);
                *e.value()
            }
            None => return 0,
        };

        let rg = self.raid_group(key);
        let first_byte_in = offset / ALLOC_UNIT;
        let last_byte_in = (offset + len - 1) / ALLOC_UNIT;

        // Copy the value into the response buffer.
        let mut copied = 0;
        for i in first_byte_in..=last_byte_in {
            let value = self.valuebuf[rg]
                .entry(i)
                .or_insert_with(|| [0; ALLOC_UNIT]);

            let base = i * ALLOC_UNIT;
            let start = base.max(offset) - base;
            let end = (base + ALLOC_UNIT).min(offset + len) - base;

            let value = &value.as_ref()[start..end];
            ptr::copy_nonoverlapping(value.as_ptr(), dst.add(copied), end - start);
            copied += end - start;
        }
        assert!(copied == len);
        len
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
                let committed_xid = self.peer_committed_xids[primary].lock().unwrap();
                while let Some(item) = wq.pop() {
                    let xid = item.0.xid;
                    if xid > *committed_xid {
                        wq.push(item);
                        break;
                    }

                    let key = item.0.meta.key;
                    let rg_first_key = (key / self.ec_config.k as u64) * self.ec_config.k as u64;
                    self.index
                        .insert(rg_first_key, (item.0.offset, item.1.len()));
                    self.update(
                        self.raid_group(key),
                        item.1.as_ptr(),
                        item.0.offset,
                        item.1.len(),
                    );
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

        let primary = (hdr.key % self.cluster.size() as u64) as usize;

        // Compute the nodes of this RAID group
        let rg = self.raid_group(hdr.key);
        let rg_parity_base = ((rg + 1) * self.ec_config.k) % self.cluster.size();

        // If this is a write and I am the primary, then it does not contain XIDs.
        // Otherwise, this request is from the primary and I need to find the XIDs.
        let mut resp_buf = req.pre_resp_buf();
        let resp_payload_len;

        if req.req_type() == RPC_DATA_WRITE {
            // Simulate slow down.
            if self.slow_down[self.cluster.rank()].load(Ordering::Relaxed) {
                // Simulate slow down.
                async_sleep(Duration::from_millis(1)).await;
            }

            resp_payload_len = 0;
            if primary == self.cluster.rank() {
                log::trace!("received write request for key {}", hdr.key);
                assert!(req_buf.len() >= <ObjMeta<K>>::len() + hdr.len as usize);

                // let (mut t_coding, mut t_repl, mut t_kvs) = (0u64, 0u64, 0u64);
                // let start_time = Instant::now();

                // Compute diff
                let (offset, diff) = match self.get(hdr.key) {
                    Some(val) => {
                        let (offset, _) = *self.index.get(&hdr.key).unwrap().value();
                        let len = val.len().min(hdr.len as usize);

                        // Do XOR only for the common part
                        let mut diff = Vec::with_capacity(hdr.len as usize);
                        isal::xor_compute(
                            &mut diff.spare_capacity_mut()[..],
                            &[&val[..len], unsafe {
                                &req_buf.as_slice()[<ObjMeta<K>>::len()..<ObjMeta<K>>::len() + len]
                            }],
                        );
                        if len <= hdr.len as usize {
                            self.alloc[rg]
                                .alloc(hdr.len as usize - len)
                                .expect("no memory");
                            unsafe {
                                ptr::write_bytes(
                                    diff.spare_capacity_mut().as_mut_ptr().add(len),
                                    0,
                                    hdr.len as usize - len,
                                )
                            };
                        }
                        unsafe { diff.set_len(hdr.len as usize) };
                        (offset, diff)
                    }
                    None => {
                        let offset = self.insert(
                            hdr.key,
                            unsafe { req_buf.as_ptr().add(<ObjMeta<K>>::len()) },
                            hdr.len as usize,
                        );

                        let mut diff = vec![0; hdr.len as usize];
                        diff.copy_from_slice(unsafe {
                            &req_buf.as_slice()
                                [<ObjMeta<K>>::len()..<ObjMeta<K>>::len() + hdr.len as usize]
                        });
                        (offset, diff)
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
                    isal::pq_update(
                        &diff,
                        (hdr.key % self.ec_config.k as u64) as usize,
                        self.ec_config.k,
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
                        ptr::write_unaligned(
                            req_bufs[i].as_ptr() as *mut RaidRpcRequest<K>,
                            RaidRpcRequest::new(hdr, xid, committed, offset),
                        )
                    };
                    repl_futs.push(sess.request(RPC_DATA_WRITE, nested_req_buf, nested_resp_buf));
                }

                // let t3 = Instant::now();
                // t_repl += (t3 - t2).as_nanos() as u64;

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
                assert!(req_buf.len() >= <RaidRpcRequest<K>>::len());
                let raid_req =
                    unsafe { ptr::read_unaligned(req_buf.as_ptr() as *const RaidRpcRequest<K>) };
                let (xid, committed) = (raid_req.xid, raid_req.committed);

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
                self.wqs[primary].push(PendingKvEntry(raid_req, diff));
            }
        } else if req.req_type() == RPC_DATA_READ {
            resp_payload_len = if primary == self.cluster.rank() {
                // Normal read.
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

                let rg_first_key = (hdr.key / self.ec_config.k as u64) * self.ec_config.k as u64;
                let rg_first_node = (rg_first_key % self.cluster.size() as u64) as usize;

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
                            rg_first_key + (*i) as u64
                        } else {
                            rg_first_key
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
                    let mut recover_from = Vec::with_capacity(self.ec_config.k);
                    let mut recover_to = Vec::with_capacity(self.ec_config.p);

                    let mut expected_next = 0;
                    for (i, chunk) in chunks.into_iter().enumerate() {
                        let (idx, _) = recovery_src[i];
                        if idx < self.ec_config.k {
                            while expected_next < idx {
                                data.push(Vec::new());
                                recover_to.push((expected_next, vec![0; len]));
                                expected_next += 1;
                            }
                            data.push(chunk);
                            recover_from.push(idx);
                            expected_next += 1;
                        } else {
                            parity.push(chunk);
                            recover_from.push(idx);
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
                        let data_ref = recover_from
                            .iter()
                            .map(|&idx| {
                                if idx < self.ec_config.k {
                                    data[idx].as_ref()
                                } else {
                                    parity[(idx - self.ec_config.k).min(parity.len() - 1)].as_ref()
                                }
                            })
                            .collect::<Vec<_>>();
                        let mut parity_ref = recover_to
                            .iter_mut()
                            .map(|(_, b)| b.as_mut())
                            .collect::<Vec<_>>();

                        isal::pq_encode(&data_ref, &mut parity_ref);
                        for (idx, buf) in recover_to {
                            if idx < self.ec_config.k {
                                data[idx] = buf;
                            }
                        }
                    }

                    // Now all lost chunks should be recovered. I just find the one I want.
                    let my_idx = (0..self.ec_config.k)
                        .find(|&i| (rg_first_key + i as u64) == hdr.key)
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
    let nthreads = NThreads::load_toml_file(&args.config_file)?;
    if cluster.rank() == 0 {
        log::info!("===== P+Q baseline =====");
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
