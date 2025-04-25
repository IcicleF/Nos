use super::{Key, Version};
use std::{mem, ptr};

/// Object metadata.
///
/// This structure is intended to be sent over network.
/// It is trivially copyable and has a C representation to ensure interoperability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ObjMeta<K>
where
    K: Key,
{
    /// Key of the object.
    pub key: K,

    /// Length of the value in bytes.
    pub len: u32,

    /// Version number.
    pub ver: Version,
}

#[inline(never)]
#[cold]
fn alignment_check_fail(pointer: *const u8, alignment: usize) {
    panic!(
        "object metadata pointer {:p} is not aligned (align = {})",
        pointer, alignment
    );
}

impl<K> ObjMeta<K>
where
    K: Key,
{
    /// Get the size of the object metadata.
    pub const fn len() -> usize {
        mem::size_of::<Self>()
    }

    /// Creates a new object metadata.
    /// Old length and version fields are omitted.
    pub fn new(key: K, len: usize, ver: Version) -> Self {
        debug_assert!(len <= u32::MAX as usize, "object too large");
        Self {
            key,
            len: len as u32,
            ver,
        }
    }

    /// Construct a reference of object metadata from raw pointer.
    ///
    /// # Safety
    ///
    /// The pointer must point to the start address of a valid [`ObjMeta<K>`] object.
    ///
    /// ## Panics
    ///
    /// Panic if the pointer is not aligned.
    pub unsafe fn from_ptr(src: *const u8) -> Self {
        if src as usize % mem::align_of::<Self>() != 0 {
            alignment_check_fail(src, mem::align_of::<Self>());
        }
        ptr::read_volatile(src as *const Self)
    }
}
