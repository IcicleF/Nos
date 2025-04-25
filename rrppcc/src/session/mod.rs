mod handle;
mod sslot;

use std::array;
use std::collections::VecDeque;
use std::net::SocketAddr;

use rrddmma::rdma::qp::{Qp, QpPeer};

pub use self::handle::*;
pub(crate) use self::sslot::*;
use crate::msgbuf::MsgBuf;
use crate::pkthdr::PacketHeader;
use crate::rpc::RpcInterior;
use crate::type_alias::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionRole {
    Client,
    Server,
}

pub(crate) struct PendingRequest {
    /// Request PacketHeader.
    pub pkthdr: PacketHeader,

    /// Request MsgBuf.
    pub req: MsgBuf,

    /// Response MsgBuf.
    pub resp: MsgBuf,

    /// Expected request index, only for correctness check.
    pub expected_req_idx: ReqIdx,

    /// Aborted flag.
    pub aborted: bool,
}

pub(crate) const ACTIVE_REQ_WINDOW: usize = 8;

pub(crate) struct Session {
    /// Role of this session.
    role: SessionRole,
    /// Connection status.
    /// `None` indicates connection in progress, `Some(true)` indicates connected,
    /// `Some(false)` indicates disconnected or connection refused.
    pub connected: Option<bool>,

    /// Remote peer's Nexus URI.
    pub peer_uri: SocketAddr,
    /// Remote peer's Rpc ID.
    pub peer_rpc_id: RpcId,
    /// Remote peer's session ID.
    pub peer_sess_id: SessId,
    /// Remote peer routing information (for UD transport).
    pub peer: Option<QpPeer>,
    /// Connection to the remote peer.
    pub rc_qp: Qp,

    /// Session request slots.
    /// Pinned in heap to avoid moving around, invalidating pointers.
    pub slots: Box<[SSlot; ACTIVE_REQ_WINDOW]>,
    /// Available SSlot index list.
    pub avail_slots: VecDeque<usize>,
    /// Queue for requests that are waiting for credits.
    pub req_backlog: [VecDeque<PendingRequest>; ACTIVE_REQ_WINDOW],
}

impl Session {
    /// Create a new session with empty peer information.
    pub fn new(state: &mut RpcInterior, role: SessionRole, rc_qp: Qp) -> Self {
        // Initialize SSlots.
        let initial_psn_base = match role {
            SessionRole::Client => ACTIVE_REQ_WINDOW,
            SessionRole::Server => 0,
        };
        let slots = array::from_fn(|i| SSlot::new(state, role, (i + initial_psn_base) as _));

        Self {
            role,
            connected: Some(false),

            peer_uri: SocketAddr::from(([0, 0, 0, 0], 0)),
            peer_rpc_id: 0,
            peer_sess_id: 0,
            peer: None,
            rc_qp,

            slots: Box::new(slots),
            avail_slots: (0..ACTIVE_REQ_WINDOW).collect(),
            req_backlog: array::from_fn(|_| VecDeque::new()),
        }
    }

    /// Return `true` if this session is a client, otherwise server.
    #[inline(always)]
    pub fn is_client(&self) -> bool {
        self.role == SessionRole::Client
    }

    /// Return `true` if this session is connected.
    #[inline(always)]
    pub fn is_connected(&self) -> bool {
        self.connected.unwrap_or(false)
    }
}
