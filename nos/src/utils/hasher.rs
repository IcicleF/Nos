#[cfg(feature = "use_ahash")]
pub use ahash::RandomState;

#[cfg(feature = "use_fxhash")]
pub use fxhash::FxBuildHasher as RandomState;

#[cfg(not(any(feature = "use_ahash", feature = "use_fxhash")))]
pub use std::collections::hash_map::RandomState;
