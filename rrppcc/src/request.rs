//! Define `Request`, an awaitable object that resolves to an RPC response.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use quanta::Instant;

use crate::msgbuf::MsgBuf;
use crate::rpc::Rpc;
use crate::type_alias::*;
use crate::util::likely::*;

/// An awaitable object that represents an incompleted RPC request.
///
/// Resolves to nothing when the request is completed.
/// The user should find the response in the buffer they provided when sending the request.
pub struct Request<'a> {
    /// Pointer to the `Rpc` instance that this request belongs to.
    rpc: &'a Rpc,

    /// Pointer to the `MsgBuf` that the user designated for the response.
    resp_buf: &'a mut MsgBuf,

    /// Session ID.
    sess_id: SessId,

    /// SSlot index.
    sslot_idx: usize,

    /// Request index.
    req_idx: ReqIdx,

    /// Time of the last transmission of the request.
    start_time: Instant,
}

impl<'a> Request<'a> {
    /// Create a new request.
    #[inline(always)]
    pub(crate) fn new(
        rpc: &'a Rpc,
        resp_buf: &'a mut MsgBuf,
        sess_id: SessId,
        sslot_idx: usize,
        req_idx: ReqIdx,
    ) -> Self {
        Self {
            rpc,
            resp_buf,
            sess_id,
            sslot_idx,
            req_idx,
            start_time: Instant::recent(),
        }
    }
}

impl Request<'_> {
    /// Retransmit the request if the response is not received within 20ms.
    pub const RETX_TIMEOUT: Duration = Duration::from_millis(20);

    /// Immediately abort the request, no matter whether it has completed or not.
    /// If the request has not finished, the response will be discarded in the future.
    /// Consume the request object, so that further polling is impossible.
    ///
    /// If the request is in the pending queue and not even started, aborting it leads
    /// to a traversal of the pending queue, which is not very efficient.
    #[inline]
    pub fn abort(self) {
        self.rpc
            .abort_request((self.sess_id, self.sslot_idx, self.req_idx));
    }
}

impl<'a> Future for Request<'a> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // If the request has finished, return the response.
        let need_re_tx = Instant::recent() - self.start_time > Self::RETX_TIMEOUT;
        if self.rpc.check_request_completion(
            (self.sess_id, self.sslot_idx, self.req_idx),
            self.resp_buf,
            need_re_tx,
        ) {
            return Poll::Ready(());
        }
        if unlikely(need_re_tx) {
            self.start_time = Instant::recent();
        }

        // Otherwise, try to make some progress.
        self.rpc.progress();
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}
