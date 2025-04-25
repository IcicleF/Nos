//! Define types used by RPC request handlers.

use std::future::Future;
use std::pin::Pin;

use crate::msgbuf::MsgBuf;
use crate::rpc::Rpc;
use crate::session::SSlot;
use crate::type_alias::*;

/// RPC request handler function return type.
pub(crate) type ReqHandlerFuture = Pin<Box<dyn Future<Output = MsgBuf> + 'static>>;

/// RPC request handler function trait.
///
/// Every handler function take two parameters.
/// The first is a [`Pin<&Rpc>`], which refers to the [`Rpc`] instance that calls
/// this handler function due to a client RPC request.
/// The second is a  [`ReqHandle`] that contains pointers to.
/// The handler should` ` return a future that resolves to the length of the response.
/// The response should be filled in the buffer pointed by `resp_buf`.
pub(crate) type ReqHandler = Box<dyn Fn(RequestHandle) -> ReqHandlerFuture + 'static>;

/// RPC request handle exposed to request handlers.
///
/// An instance of this type passed to a request handler function only represents
/// valid state during the lifetime of the handler function call, and must be dropped
/// when/before the handler returns. Failing to do so will result in a panic.
pub struct RequestHandle {
    /// Pointer to the `Rpc` instance that calls this handler function.
    rpc: *const Rpc,

    /// Pointer to the SSlot of this request.
    sslot: *mut SSlot,
}

impl RequestHandle {
    /// Construct a request handle.
    #[inline(always)]
    pub(crate) fn new<'a>(rpc: &'a Rpc, sslot: &'a mut SSlot) -> Self {
        // Mark the SSlot as having a handle.
        debug_assert!(!sslot.has_handle);
        sslot.has_handle = true;

        Self { rpc, sslot }
    }
}

impl RequestHandle {
    /// Return the `Rpc` instance that called this handler function.
    #[inline(always)]
    pub fn rpc(&self) -> &Rpc {
        // SAFETY: the pointer is always valid.
        unsafe { &*self.rpc }
    }

    /// Return the type of this request.
    #[inline(always)]
    pub fn req_type(&self) -> ReqType {
        // SAFETY: the pointer is always valid.
        unsafe { (*self.sslot).req_type }
    }

    /// Return the request buffer.
    #[inline(always)]
    pub fn req_buf(&self) -> &MsgBuf {
        // SAFETY: the pointer is always valid.
        unsafe { (*self.sslot).req_buf() }
    }

    /// Return the prepared response buffer.
    ///
    /// This buffer can only accommodate MTU-sized data (usually 4KiB). If you
    /// need larger responses, you should use `Rpc::alloc_msgbuf()`.
    #[inline(always)]
    pub fn pre_resp_buf(&self) -> MsgBuf {
        // SAFETY: the pointer is always valid.
        unsafe { (*self.sslot).pre_resp_msgbuf.clone_borrowed() }
    }
}

impl Drop for RequestHandle {
    fn drop(&mut self) {
        // SAFETY: the pointer is always valid.
        let sslot = unsafe { &mut *self.sslot };

        // Mark the SSlot as not having a handle.
        debug_assert!(sslot.has_handle);
        sslot.has_handle = false;
    }
}
