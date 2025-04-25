//! Implementation of a fake RefCell that bypasses any safety checks.
#![allow(dead_code)]

use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};

/// A fake RefCell that bypasses any safety checks.
///
/// This type is unsafe to use. It wraps unsafe code in safe interfaces without
/// any checks. Therefore, it is only for internal use.
#[repr(transparent)]
pub(crate) struct UnsafeRefCell<T> {
    value: UnsafeCell<T>,
}

impl<T> UnsafeRefCell<T> {
    /// Construct a new fake RefCell.
    #[inline(always)]
    pub(crate) const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    /// Get a immutable reference to the inner value.
    #[inline(always)]
    pub(crate) fn borrow(&self) -> &T {
        unsafe { &*self.value.get() }
    }

    /// Get a mutable reference to the inner value.
    /// Wrap it in a `RefMut` to make other code unchanged.
    #[inline(always)]
    pub(crate) fn borrow_mut(&self) -> RefMut<T> {
        RefMut {
            value: unsafe { &mut *self.value.get() },
        }
    }
}

/// A mutable reference into the value of a [`UnsafeRefCell`].
#[repr(transparent)]
pub(crate) struct RefMut<'a, T> {
    value: &'a mut T,
}

impl<T> Deref for RefMut<'_, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<T> DerefMut for RefMut<'_, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}
