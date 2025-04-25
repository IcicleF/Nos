pub(crate) mod buddy;
pub(crate) mod buffer;
pub(crate) mod huge_alloc;
pub(crate) mod likely;
pub(crate) mod slab;

#[cfg(not(debug_assertions))]
pub(crate) mod unsafe_refcell;
