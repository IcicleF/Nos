use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use quanta::Instant;
use rmp_serde as rmps;

use crate::msgbuf::MsgBuf;
use crate::nexus::{SmEvent, SmEventDetails};
use crate::request::Request;
use crate::rpc::Rpc;
use crate::type_alias::*;
use crate::util::likely::*;

/// Handle to a session that points to a specific remote `Rpc` endpoint.
#[derive(Clone, Copy)]
pub struct SessionHandle<'r> {
    /// The RPC instance that owns this session.
    rpc: &'r Rpc,

    /// Session ID.
    sess_id: SessId,

    /// Peer Nexus SM URI.
    remote_uri: SocketAddr,

    /// Peer Rpc ID.
    remote_rpc_id: RpcId,
}

impl<'r> SessionHandle<'r> {
    /// Create a new session handle.
    #[inline(always)]
    pub(crate) fn new(
        rpc: &'r Rpc,
        sess_id: SessId,
        remote_uri: SocketAddr,
        remote_rpc_id: RpcId,
    ) -> Self {
        Self {
            rpc,
            sess_id,
            remote_uri,
            remote_rpc_id,
        }
    }
}

impl<'r> SessionHandle<'r> {
    /// Return the session ID.
    #[inline(always)]
    pub fn id(&self) -> SessId {
        self.sess_id
    }

    /// Return the remote peer.
    #[inline(always)]
    pub fn remote(&self) -> (SocketAddr, RpcId) {
        (self.remote_uri, self.remote_rpc_id)
    }

    /// Return the RPC instance that owns this session.
    #[inline(always)]
    pub fn rpc(&self) -> &'r Rpc {
        self.rpc
    }

    /// Return `true` if the session is connected.
    #[inline]
    pub fn is_connected(&self) -> bool {
        self.rpc
            .session_connection_state(self.sess_id)
            .unwrap_or(false)
    }

    /// Connect the session to the remote peer.
    ///
    /// This method returns an awaitable object that resolves to a
    /// boolean value indicating whether the connection is successful.
    /// Note that connection will not start until the first poll.
    pub fn connect<'a>(&'a self) -> impl Future<Output = bool> + 'a
    where
        'r: 'a,
    {
        let msg = if likely(!self.is_connected()) {
            // Mark the session as connecting.
            let (cli_ud_ep, cli_sess_rc_ep) = self.rpc.mark_session_connecting(self.sess_id);

            // Send a connect request to the remote peer.
            let event = SmEvent {
                src_rpc_id: self.rpc.id(),
                dst_rpc_id: self.remote_rpc_id,
                details: SmEventDetails::ConnectRequest {
                    cli_uri: self.rpc.nexus().uri(),
                    cli_ud_ep,
                    cli_sess_id: self.sess_id,
                    cli_sess_rc_ep,
                },
            };
            rmps::to_vec(&event).unwrap()
        } else {
            Vec::new()
        };

        SessionConnect {
            rpc: self.rpc,
            sess_id: self.sess_id,
            remote_uri: self.remote_uri,
            msg,
            start_time: Instant::now() - SessionConnect::TIMEOUT,
        }
    }

    /// Send a request in this session.
    ///
    /// When the return value is resolved, the response buffer will also be resized
    /// to the proper length. Note that truncation may occur if the response buffer
    /// is too small, and there will not be any warning.
    pub fn request<'a>(
        &'a self,
        req_type: ReqType,
        req_msgbuf: &'a MsgBuf,
        resp_msgbuf: &'a mut MsgBuf,
    ) -> Request<'a>
    where
        'r: 'a,
    {
        self.rpc
            .enqueue_request(self.sess_id, req_type, req_msgbuf, resp_msgbuf)
    }
}

/// Session connection awaitable.
struct SessionConnect<'a> {
    /// The `Rpc` instance that owns this session.
    rpc: &'a Rpc,

    /// Session ID.
    sess_id: SessId,

    /// URI of the remote peer's Nexus.
    remote_uri: SocketAddr,

    /// The message to send to the remote peer's Nexus.
    msg: Vec<u8>,

    /// The time when the last SM packet is sent.
    start_time: Instant,
}

impl SessionConnect<'_> {
    /// Connection timeout duration.
    pub const TIMEOUT: Duration = Duration::from_millis(100);
}

impl Future for SessionConnect<'_> {
    type Output = bool;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Check if the session connection state has already been determined.
        if let Some(result) = self.rpc.session_connection_state(self.sess_id) {
            Poll::Ready(result)
        } else {
            // Send the connect request to the remote peer if timeout.
            if self.start_time.elapsed() >= Self::TIMEOUT {
                self.rpc
                    .sm_tx
                    .send_to(&self.msg, self.remote_uri)
                    .expect("failed to send ConnectRequest");
                self.start_time = Instant::now();
            }

            self.rpc.progress();
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}
