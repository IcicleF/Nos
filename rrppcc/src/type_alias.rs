//! Type aliases used in this library.

/// [`u16`]: Rpc identifier.
/// Note that this type distinguishes `Rpc` instances (control-plane), not RPC requests (data-plane).
pub type RpcId = u16;

/// [`u32`]: Session identifier.
pub type SessId = u32;

/// [`u8`]: Request type identifier.
pub type ReqType = u8;

/// [`u64`] (62 bits valid): Request index within a session.
pub(crate) type ReqIdx = u64;
