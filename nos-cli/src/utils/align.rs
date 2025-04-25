#![allow(dead_code)]

use std::mem;
use std::ops::*;

#[inline]
pub fn make_aligned<T>(buf: *const u8) -> *const T {
    let align = mem::align_of::<T>();
    let mask = align - 1;
    let aligned = (buf as usize + mask) & !mask;
    aligned as *const T
}

#[inline]
pub fn make_aligned_mut<T>(buf: *mut u8) -> *mut T {
    make_aligned::<T>(buf) as *mut T
}

/// A type wrapper that has at least cacheline (64B) alignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(align(64))]
pub struct ClAligned<T>(T);

impl<T> ClAligned<T> {
    pub fn new(t: T) -> Self {
        Self(t)
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for ClAligned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for ClAligned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> From<T> for ClAligned<T> {
    fn from(t: T) -> Self {
        Self::new(t)
    }
}
