use crate::util::likely::*;
use libc::*;
use std::ptr;

const HUGE_PAGE_SIZE: usize = 1 << 21;

enum AllocType {
    Mmap,
    Malloc,
}

pub(crate) struct HugeAlloc {
    pub ptr: *mut u8,
    pub len: usize,
    alloc_type: AllocType,
}

unsafe impl Send for HugeAlloc {}
unsafe impl Sync for HugeAlloc {}

impl Drop for HugeAlloc {
    fn drop(&mut self) {
        // SAFETY: FFI.
        unsafe {
            match self.alloc_type {
                AllocType::Mmap => assert!(
                    munmap(self.ptr as *mut c_void, self.len) == 0,
                    "munmap failed"
                ),
                AllocType::Malloc => free(self.ptr as *mut c_void),
            }
        };
    }
}

#[inline]
fn alloc_mmap(len: usize, flags: i32) -> *mut u8 {
    // SAFETY: FFI.
    let ret = unsafe {
        mmap(
            ptr::null_mut(),
            len,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS | flags,
            -1,
            0,
        ) as *mut u8
    };

    if ret != MAP_FAILED as _ {
        ret
    } else {
        ptr::null_mut()
    }
}

#[inline]
fn alloc_memalign(len: usize, align: usize) -> *mut u8 {
    let mut ptr = ptr::null_mut();
    // SAFETY: FFI.
    let ret = unsafe { posix_memalign(&mut ptr, align, len) };
    if likely(ret == 0) {
        ptr as _
    } else {
        ptr::null_mut()
    }
}

/// Allocate memory.
pub(crate) fn alloc_raw(len: usize) -> HugeAlloc {
    // Roundup to huge page size.
    let len = (len + HUGE_PAGE_SIZE - 1) & !(HUGE_PAGE_SIZE - 1);

    // 1. Try to allocate huge page.
    let ptr = alloc_mmap(len, MAP_HUGETLB);
    if !ptr.is_null() {
        return HugeAlloc {
            ptr,
            len,
            alloc_type: AllocType::Mmap,
        };
    }

    log::warn!(
        "failed to mmap {}MB hugepages, trying normal pages; performance can be low.",
        len >> 20
    );

    // 2. Try to allocate normal page.
    let ptr = alloc_mmap(len, 0);
    if likely(!ptr.is_null()) {
        return HugeAlloc {
            ptr,
            len,
            alloc_type: AllocType::Mmap,
        };
    }

    log::warn!(
        "failed to mmap {}MB normal pages, trying posix_memalign; performance can be low.",
        len >> 20
    );

    // 3. Try to posix_memalign, align to page size.
    let ptr = alloc_memalign(len, 1 << 12);
    if likely(!ptr.is_null()) {
        return HugeAlloc {
            ptr,
            len,
            alloc_type: AllocType::Malloc,
        };
    }

    panic!("failed to allocate {}MB memory", len >> 20);
}
