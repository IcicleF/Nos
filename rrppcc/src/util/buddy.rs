use std::ptr::NonNull;
use std::rc::Rc;

// #[cfg(debug_assertions)]
use std::cell::RefCell as InteriorCell;

// #[cfg(not(debug_assertions))]
// use crate::util::unsafe_refcell::UnsafeRefCell as InteriorCell;

use crate::transport::{LKey, RKey, UdTransport};
use crate::util::{buffer::*, huge_alloc::*};

/// A buffer that represents a piece of unallocated memory in the buddy allocator.
///
/// This type does not contain any length information, as the place it resides in
/// should contain such information.
struct InBuddyBuffer {
    /// Start address of the buffer.
    buf: NonNull<u8>,

    /// Local key.
    lkey: LKey,

    /// Remote key.
    rkey: RKey,
}

impl InBuddyBuffer {
    /// Create a new buffer.
    #[inline(always)]
    pub fn new(buf: NonNull<u8>, lkey: LKey, rkey: RKey) -> Self {
        Self { buf, lkey, rkey }
    }

    /// Return a new buffer that starts at an offset to the current one.
    ///
    /// # Safety
    ///
    /// Same as [`pointer::add()`](https://doc.rust-lang.org/std/primitive.pointer.html#method.add).
    #[inline(always)]
    pub unsafe fn offset(&self, offset: usize) -> InBuddyBuffer {
        InBuddyBuffer {
            buf: NonNull::new_unchecked(self.buf.as_ptr().add(offset)),
            lkey: self.lkey,
            rkey: self.rkey,
        }
    }
}

/// The true buddy allocator, but it cannot be directly exposed to the crate
/// since there are pointer-based accesses triggered by dropping a `MsgBuf`.
struct BuddyAllocatorInner {
    /// Buddy system.
    buddy: [Vec<InBuddyBuffer>; Self::NUM_CLASSES],

    /// Allocated memory registry.
    #[allow(dead_code)]
    mem_registry: Vec<HugeAlloc>,

    /// Next allocation size.
    next_alloc: usize,
}

impl BuddyAllocatorInner {
    const MIN_ALLOC_SIZE: usize = 1 << 6;
    const MAX_ALLOC_SIZE: usize = 1 << 24;
    const NUM_CLASSES: usize =
        (Self::MAX_ALLOC_SIZE / Self::MIN_ALLOC_SIZE).trailing_zeros() as usize + 1;

    /// Current buffer exhausted (for some size class allocation), so allocate new memory.
    #[cold]
    fn reserve_memory(&mut self, tp: &mut UdTransport) {
        let len = self.next_alloc;
        self.next_alloc *= 2;
        debug_assert!(len % Self::MAX_ALLOC_SIZE == 0);

        let mem = alloc_raw(len);
        let (lkey, rkey) = unsafe { tp.reg_mem(mem.ptr, len) };

        for i in 0..(len / Self::MAX_ALLOC_SIZE) {
            self.buddy[Self::NUM_CLASSES - 1].push(InBuddyBuffer::new(
                unsafe { NonNull::new_unchecked(mem.ptr.add(i * Self::MAX_ALLOC_SIZE)) },
                lkey,
                rkey,
            ));
        }
        self.mem_registry.push(mem);
    }

    /// Return the size of a given class.
    #[inline]
    const fn size_of_class(class: usize) -> usize {
        Self::MIN_ALLOC_SIZE << class
    }

    /// Return the smallest class that can accommodate a given size.
    #[inline]
    const fn class_of(len: usize) -> usize {
        let len = len.next_power_of_two();
        if len < Self::MIN_ALLOC_SIZE {
            0
        } else {
            (len / Self::MIN_ALLOC_SIZE).trailing_zeros() as usize
        }
    }

    /// Split a buffer of the given class into two buffers of the next lower class.
    #[inline]
    fn split(&mut self, class: usize) {
        debug_assert!((1..Self::NUM_CLASSES).contains(&class));
        debug_assert!(!self.buddy[class].is_empty());

        let size_after_split = Self::size_of_class(class - 1);
        let buf1 = self.buddy[class].pop().unwrap();

        // SAFETY: guaranteed not null, within the same allocated memory buffer.
        let buf2 = unsafe { buf1.offset(size_after_split) };

        self.buddy[class - 1].push(buf1);
        self.buddy[class - 1].push(buf2);
    }
}

impl BuddyAllocatorInner {
    /// Create a new buddy allocator with no pre-allocation.
    pub fn new() -> Self {
        Self {
            buddy: Default::default(),
            mem_registry: Vec::new(),
            next_alloc: Self::MAX_ALLOC_SIZE,
        }
    }

    /// Allocate a new buffer with at least the given length.
    pub fn alloc(
        &mut self,
        len: usize,
        tp: &mut UdTransport,
        owner: &Rc<BuddyAllocator>,
    ) -> Buffer {
        assert!(
            len <= Self::MAX_ALLOC_SIZE,
            "requested buffer too large (maximum: {}MB)",
            Self::MAX_ALLOC_SIZE >> 20
        );

        let class = Self::class_of(len);
        if self.buddy[class].is_empty() {
            let higher_class =
                ((class + 1)..Self::NUM_CLASSES).find(|&c| !self.buddy[c].is_empty());
            let higher_class = higher_class.unwrap_or_else(|| {
                self.reserve_memory(tp);
                Self::NUM_CLASSES - 1
            });

            debug_assert!(!self.buddy[higher_class].is_empty());
            for i in ((class + 1)..=higher_class).rev() {
                self.split(i);
            }
            debug_assert!(!self.buddy[class].is_empty());
        }
        let buf = self.buddy[class].pop().unwrap();
        Buffer::real(
            buf.buf,
            Self::size_of_class(class),
            buf.lkey,
            buf.rkey,
            Some(owner.clone()),
        )
    }

    /// Free a buffer.
    /// This does not actually free the memory, but returns it to the buddy allocator.
    pub fn free(&mut self, buf: &Buffer) {
        let class = Self::class_of(buf.len());
        self.buddy[class].push(InBuddyBuffer::new(
            // SAFETY: `buf.as_ptr()` returns the raw pointer stored in `NonNull`.
            unsafe { NonNull::new_unchecked(buf.as_ptr()) },
            buf.lkey(),
            buf.rkey(),
        ));
    }
}

/// A buddy allocator that never combines buddies.
#[repr(transparent)]
pub(crate) struct BuddyAllocator {
    inner: InteriorCell<BuddyAllocatorInner>,
}

impl BuddyAllocator {
    /// The maximum allocation size, 16MB.
    pub const MAX_ALLOC_SIZE: usize = 1 << 24;

    /// Create a new buddy allocator with no pre-allocation.
    pub fn new() -> Self {
        Self {
            inner: InteriorCell::new(BuddyAllocatorInner::new()),
        }
    }

    /// Allocate a new buffer with at least the given length.
    pub fn alloc(self: &Rc<Self>, len: usize, tp: &mut UdTransport) -> Buffer {
        self.inner.borrow_mut().alloc(len, tp, self)
    }

    /// Free a buffer.
    /// This does not actually free the memory, but returns it to the buddy allocator.
    pub fn free(&self, buf: &Buffer) {
        self.inner.borrow_mut().free(buf)
    }
}
