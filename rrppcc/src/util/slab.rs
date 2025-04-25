use std::marker::PhantomData;
use std::mem;
use std::ptr::{self, NonNull};

use crate::transport::{LKey, UdTransport};
use crate::util::{buffer::*, huge_alloc::*};

/// A slab allocator that allocates fixed slabs of registered memory and never
/// frees them.
pub(crate) struct SlabAllocator<T> {
    /// Allocated memory registry.
    mem_registry: Vec<HugeAlloc>,

    /// Allocation length.
    len: usize,

    /// Next allocation site.
    next: *mut u8,

    /// LKey of the next allocated memory.
    lkey: LKey,

    /// Phantom data.
    _marker: PhantomData<T>,
}

impl<T> SlabAllocator<T> {
    /// Determine whether the current HugeAlloc is used up.
    #[inline(always)]
    fn is_used_up(&self) -> bool {
        self.next.is_null()
            || self.mem_registry.is_empty()
            || self.next == unsafe { self.mem_registry.last().unwrap().ptr.add(Self::ALLOC_SIZE) }
    }

    /// Current buffer exhausted (for some size class allocation), so allocate new memory.
    #[cold]
    fn reserve_memory(&mut self, tp: &mut UdTransport) {
        let mem = alloc_raw(Self::ALLOC_SIZE);
        let (lkey, _) = unsafe { tp.reg_mem(mem.ptr, Self::ALLOC_SIZE) };

        self.next = mem.ptr;
        self.lkey = lkey;
        self.mem_registry.push(mem);
    }
}

impl<T> SlabAllocator<T> {
    const ALLOC_SIZE: usize = 1 << 20;

    /// Create a new slab allocator with no pre-allocation.
    pub fn new() -> Self {
        let len = mem::size_of::<T>().next_power_of_two();
        assert!(
            len <= Self::ALLOC_SIZE,
            "SlabAllocator: size class too large"
        );

        Self {
            mem_registry: Vec::new(),
            len,
            next: ptr::null_mut(),
            lkey: 0,
            _marker: PhantomData,
        }
    }

    /// Allocate a new slab.
    pub fn alloc(&mut self, tp: &mut UdTransport) -> Buffer {
        if self.is_used_up() {
            self.reserve_memory(tp);
        }

        let addr = self.next;
        // SAFETY: within the same allocated buffer.
        self.next = unsafe { self.next.add(self.len) };
        Buffer::real(NonNull::new(addr).unwrap(), self.len, self.lkey, 0, None)
    }
}
