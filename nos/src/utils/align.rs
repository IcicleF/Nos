use std::mem;

/// Return the smallest address equal to or larger than `buf` that is aligned for type `T`.
#[inline]
pub fn make_aligned<T>(buf: *const u8) -> *const T {
    let align = mem::align_of::<T>();
    let mask = align - 1;
    let aligned = (buf as usize + mask) & !mask;
    aligned as *const T
}

/// Return the smallest address equal to or larger than `buf` that is aligned to the given
/// alignment.
#[inline]
pub fn make_aligned_to(buf: *const u8, alignment: usize) -> *const u8 {
    let mask = alignment - 1;
    let aligned = (buf as usize + mask) & !mask;
    aligned as *const u8
}

/// Return the smallest address equal to or larger than `buf` that is aligned for type `T`.
#[inline]
pub fn make_aligned_mut<T>(buf: *mut u8) -> *mut T {
    make_aligned::<T>(buf) as *mut T
}

/// Return the smallest address equal to or larger than `buf` that is aligned to the given
/// alignment.
#[inline]
pub fn make_aligned_mut_to(buf: *mut u8, alignment: usize) -> *mut u8 {
    let mask = alignment - 1;
    let aligned = (buf as usize + mask) & !mask;
    aligned as *mut u8
}
