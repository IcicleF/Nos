use crate::handler::ReqHandlerFuture;
use crate::type_alias::*;

/// Pending RPC request handler.
pub(super) struct PendingHandler {
    /// Session ID.
    pub sess_id: SessId,

    /// Slot ID.
    pub sslot_idx: usize,

    /// Request handler body.
    pub handler: ReqHandlerFuture,
}

impl PendingHandler {
    /// Create a new pending handler.
    #[inline]
    pub fn new(sess_id: SessId, sslot_idx: usize, handler: ReqHandlerFuture) -> Self {
        Self {
            sess_id,
            sslot_idx,
            handler,
        }
    }
}
