use std::fmt;
use std::ptr::NonNull;
use std::rc::Rc;

use crate::transport::{LKey, RKey};
use crate::util::buddy::*;

pub(crate) struct Buffer {
    /// Start address of the buffer.
    buf: NonNull<u8>,

    /// Length of the buffer.
    len: usize,

    /// Local memory handle.
    lkey: LKey,

    /// Remote memory handle.
    rkey: RKey,

    /// Pointer to the buddy allocator.
    owner: Option<Rc<BuddyAllocator>>,
}

impl fmt::Debug for Buffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let owner = if let Some(ref owner) = self.owner {
            format!("{:p}", owner)
        } else {
            "None".to_string()
        };
        f.debug_struct("Buffer")
            .field("buf", &self.buf)
            .field("len", &self.len)
            .field("lkey", &self.lkey)
            .field("rkey", &self.rkey)
            .field("owner", &owner)
            .finish()
    }
}

impl Buffer {
    /// A real buffer. Frees the buffer on drop if `owner` is `Some`.
    #[inline]
    pub fn real(
        buf: NonNull<u8>,
        len: usize,
        lkey: LKey,
        rkey: RKey,
        owner: Option<Rc<BuddyAllocator>>,
    ) -> Self {
        Self {
            buf,
            len,
            lkey,
            rkey,
            owner,
        }
    }

    /// A fake buffer that only serves to record `(LKey, RKey)`.
    #[inline]
    pub fn fake(lkey: LKey, rkey: RKey) -> Self {
        Self {
            buf: NonNull::dangling(),
            len: 0,
            lkey,
            rkey,
            owner: None,
        }
    }

    /// Get the start address of the buffer.
    #[inline(always)]
    pub fn as_ptr(&self) -> *mut u8 {
        self.buf.as_ptr()
    }

    /// Get the length of the buffer.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Get the local key of the buffer.
    #[inline(always)]
    pub fn lkey(&self) -> LKey {
        self.lkey
    }

    /// Get the remote key of the buffer.
    #[inline(always)]
    pub fn rkey(&self) -> RKey {
        self.rkey
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if let Some(ref owner) = self.owner {
            owner.free(self);
        }
    }
}
