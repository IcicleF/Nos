use std::collections::HashMap;
use std::fmt;
use std::mem::MaybeUninit;
use std::ops::{Index, IndexMut};

/// A vector that can have holes in it.
///
/// This is useful when you want a map from `usize` to some sized type, but you
/// don't want to pay the runtime costs of a [`HashMap`]. According to
/// [`criterion`](https://docs.rs/criterion/latest/criterion), this type delivers
/// an around 3x speedup than the [`HashMap`].
///
/// Boundary checking is still implemented for [`Index`] and [`IndexMut`]. The
/// checks can possibly be removed if it is proved that the whole program is
/// correct.
#[repr(transparent)]
pub struct HoledVec<T> {
    inner: Vec<(MaybeUninit<T>, bool)>,
}

impl<T> fmt::Debug for HoledVec<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut f = f.debug_list();
        for (v, is_init) in &self.inner {
            if *is_init {
                f.entry(&Some(unsafe { v.assume_init_ref() }));
            } else {
                f.entry(&<Option<&T>>::None);
            }
        }
        f.finish()
    }
}

impl<T> HoledVec<T> {
    /// Gets the given key's corresponding entry.
    #[inline]
    pub fn get(&self, key: usize) -> Option<&T> {
        if key >= self.inner.len() {
            return None;
        }

        let (v, is_init) = &self.inner[key];
        if *is_init {
            Some(unsafe { v.assume_init_ref() })
        } else {
            None
        }
    }

    /// Get the given key's corresponding entry as mutable.
    #[inline]
    pub fn get_mut(&mut self, key: usize) -> Option<&mut T> {
        if key >= self.inner.len() {
            return None;
        }

        let (v, is_init) = &mut self.inner[key];
        if *is_init {
            Some(unsafe { v.assume_init_mut() })
        } else {
            None
        }
    }

    /// Get the length of the whole vector, including the holes.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Determine if the vector is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Map the vector to another vector, consuming the input value.
    #[inline]
    pub fn map<U>(mut self, mut f: impl FnMut(T) -> U) -> HoledVec<U> {
        let mut inner = Vec::with_capacity(self.inner.len());
        for i in self.inner.drain(..) {
            if i.1 {
                inner.push((MaybeUninit::new(f(unsafe { i.0.assume_init() })), true));
            } else {
                inner.push((MaybeUninit::uninit(), false));
            }
        }
        HoledVec { inner }
    }
}

impl<T> Clone for HoledVec<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        let mut inner = Vec::with_capacity(self.inner.len());
        for (v, is_init) in &self.inner {
            if *is_init {
                inner.push((
                    MaybeUninit::new(unsafe { v.assume_init_ref().clone() }),
                    true,
                ));
            } else {
                inner.push((MaybeUninit::uninit(), false));
            }
        }
        Self { inner }
    }
}

impl<T> From<HashMap<usize, T>> for HoledVec<T> {
    fn from(value: HashMap<usize, T>) -> Self {
        let len = value.keys().max().unwrap_or(&0) + 1;
        let mut inner = Vec::with_capacity(len);
        for _ in 0..len {
            inner.push((MaybeUninit::uninit(), false));
        }
        for (k, v) in value {
            inner[k] = (MaybeUninit::new(v), true);
        }
        Self { inner }
    }
}

impl<T> From<Vec<Option<T>>> for HoledVec<T> {
    fn from(value: Vec<Option<T>>) -> Self {
        let len = value.len();
        let mut inner = Vec::with_capacity(len);
        for v in value.into_iter() {
            if let Some(v) = v {
                inner.push((MaybeUninit::new(v), true));
            } else {
                inner.push((MaybeUninit::uninit(), false));
            }
        }
        Self { inner }
    }
}

impl<T> Drop for HoledVec<T> {
    fn drop(&mut self) {
        for (v, is_init) in &mut self.inner {
            if *is_init {
                unsafe {
                    v.assume_init_drop();
                }
            }
        }
    }
}

impl<T> Index<usize> for HoledVec<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        let elem = self.inner.get(index).unwrap();
        if elem.1 {
            unsafe { elem.0.assume_init_ref() }
        } else {
            panic!("Attempted to access uninitialized value at index {}", index);
        }
    }
}

impl<T> IndexMut<usize> for HoledVec<T> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let elem = self.inner.get_mut(index).unwrap();
        if elem.1 {
            unsafe { elem.0.assume_init_mut() }
        } else {
            panic!("Attempted to access uninitialized value at index {}", index);
        }
    }
}

impl<T> HoledVec<T> {
    #[inline]
    pub fn iter(&self) -> HoledVecIter<T> {
        HoledVecIter {
            inner: self,
            index: 0,
        }
    }
}

/// Iterator over elements in a [`HoledVec`].
pub struct HoledVecIter<'a, T> {
    inner: &'a HoledVec<T>,
    index: usize,
}

impl<'a, T> Iterator for HoledVecIter<'a, T> {
    type Item = (usize, &'a T);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.inner.inner.len() {
            let elem_index = self.index;
            let elem = &self.inner.inner[elem_index];
            self.index += 1;
            if elem.1 {
                return Some((elem_index, unsafe { elem.0.assume_init_ref() }));
            }
        }
        None
    }
}

impl<T> FromIterator<(usize, T)> for HoledVec<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (usize, T)>,
    {
        let mut inner = Vec::new();
        for (k, v) in iter {
            if k >= inner.len() {
                for _ in inner.len()..(k + 1) {
                    inner.push((MaybeUninit::uninit(), false));
                }
            }
            inner[k] = (MaybeUninit::new(v), true);
        }
        Self { inner }
    }
}
