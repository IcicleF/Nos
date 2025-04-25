use std::ops::{Add, Div, Mul, Rem, Sub};
use std::{fmt, hash};

/// Key trait for the object store.
pub trait Key:
    Sized                                                   // Must have deterministic size at compile time
    + fmt::Display + fmt::Debug                             // Must be printable
    + Clone + Copy                                          // Must be trivially copyable
    + PartialOrd + Ord                                      // Must be fully ordered
    + PartialEq + Eq                                        // Must be equatable
    + Send + Sync                                           // Must be thread safe
    + hash::Hash                                            // Must be hashable
    + Add<u64, Output = Self> + Sub<u64, Output = Self>     // Must be offsetable
    + Mul<u64, Output = Self>                               // Must be scalable
    + Div<u64, Output = Self> + Rem<u64, Output = u64>      // Must be dividable so that we can put them into buckets
    + From<u64>                                             // Must be constructible from a `u64`
    + 'static                                               // Must not subject to any lifetime constraints
{
    /// Returns the additive identity element, i.e., `0`.
    /// 
    /// # Purity
    ///
    /// This function should return the same result at all times regardless of
    /// external mutable state, for example values stored in TLS or in
    /// `static mut`s.
    fn zero() -> Self;
}

/// By default, object keys are 64-bit integers.
impl Key for u64 {
    #[inline(always)]
    fn zero() -> Self {
        0
    }
}
