#![allow(private_bounds)]

mod pending;

use std::cell::RefCell;
use std::marker::PhantomPinned;
use std::net::ToSocketAddrs;
use std::net::UdpSocket;
use std::pin::Pin;
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::{array, cmp, mem, ptr};

use futures::future::Future;
use futures::task::noop_waker_ref;
use rmp_serde as rmps;
use rrddmma::rdma::qp::QpEndpoint;

use self::pending::*;
use crate::type_alias::*;
use crate::util::{buddy::*, likely::*, slab::*};
use crate::{handler::*, msgbuf::*, nexus::*, pkthdr::*, request::*, session::*, transport::*};

// #[cfg(debug_assertions)]
use std::cell::RefCell as InteriorCell;

// #[cfg(not(debug_assertions))]
// use crate::util::unsafe_refcell::UnsafeRefCell as InteriorCell;

/// Interior-mutable state of an [`Rpc`] instance.
pub(crate) struct RpcInterior {
    /// Sessions.
    sessions: Vec<Session>,

    /// RDMA UD transport layer.
    tp: UdTransport,

    /// RDMA RC transport layer.
    rc_tp: RcTransport,

    /// Pending request handlers.
    pending_handlers: Vec<PendingHandler>,

    /// Pending packet transmissions.
    pending_tx: Vec<TxItem>,

    /// SSlot PacketHeader allocator.
    slab: SlabAllocator<PacketHeader>,

    /// Buffer allocator.
    /// For safety, this is the last one to drop.
    buddy: Rc<BuddyAllocator>,
}

impl RpcInterior {
    /// Drain the pending TX queue, start their transmission on the transport.
    /// Also drain the send DMA queue if `drain_dma_queue` is set.
    fn drain_tx_batch(&mut self, drain_dma_queue: bool) {
        if !self.pending_tx.is_empty() {
            // SAFETY: items in `pending_tx` all points to valid peers and `MsgBuf`s,
            // which is guaranteed by `process_rx()` and `process_pending_handlers()`.
            unsafe { self.tp.tx_burst(&self.pending_tx, drain_dma_queue) };
            self.pending_tx.clear();
        }
    }
}

impl RpcInterior {
    /// Allocate a `MsgBuf` that can accommodate at least `len` bytes of
    /// application data. The allocated `MsgBuf` will have an initial length
    /// of `len`, but its content are uninitialized.
    #[inline]
    pub fn alloc_msgbuf(&mut self, len: usize) -> MsgBuf {
        // SAFETY: `self.allocator` is guaranteed to be valid.
        let buf_len = MsgBuf::buf_len(len);
        let buf = self.buddy.alloc(buf_len, &mut self.tp);
        MsgBuf::owned(buf, len)
    }

    /// Allocate a `MsgBuf` for a packet header.
    #[inline]
    pub fn alloc_pkthdr_buf(&mut self) -> MsgBuf {
        let buf = self.slab.alloc(&mut self.tp);
        debug_assert!((buf.as_ptr() as usize) % mem::align_of::<PacketHeader>() == 0);
        MsgBuf::owned_immutable(buf, mem::size_of::<PacketHeader>())
    }
}

/// Thread-local RPC endpoint.
///
/// This type is `Send + Sync` to make compiler happy, but it is actually not.
/// You may not create or acquire sessions, allocate message buffers, or make
/// progress from other threads except the one that created the `Rpc`. This is
/// enforced at runtime and will panic if violated.
pub struct Rpc {
    /// ID of this RPC instance.
    id: RpcId,
    /// Nexus this RPC is bound to.
    nexus: Arc<Nexus>,

    /// Session management packet sender.
    /// Use a independent socket, no need to delegate to the Nexus.
    pub(crate) sm_tx: UdpSocket,
    /// Session management event receiver.
    sm_rx: SmEventRx,

    /// Interior-mutable state of this RPC.
    /// Depending on the build mode, this field is either the checked [`RefCell`]
    /// or the unchecked [`UnsafeCell`](std::cell::UnsafeCell) with some wrappings.
    state: InteriorCell<RpcInterior>,

    /// RPC handlers.
    handlers: [Option<ReqHandler>; ReqType::MAX as usize + 1],

    /// A flag indicating whether this RPC is progressing.
    ///
    /// This flag exists because we have to leave `state` and `pending_tx` unborrowed
    /// when entering request handler contexts, but we still want to prevent reentrances
    /// of `progress()`.
    progressing: RefCell<()>,

    /// Pinning marker to prevent moving around.
    _pinned: PhantomPinned,
}

/// A macro that handles response transmission logic.
/// - `ENQUEUE`: normal response enqueue, after calling the handler.
/// - `RETRANSMIT`: detect and retransmission of generated response, before even
///                 calling the handler.
///
/// This is a macro instead of a method becuase the latter will make the borrow
/// checker very unhappy.
macro_rules! do_response_tx {
    (ENQUEUE, $state:ident, $sess:ident, $sslot:ident, $resp:ident) => {
        // Write the packet header of the response MsgBuf.
        unsafe {
            ptr::write_volatile(
                $sslot.pkthdr.as_ptr() as *mut PacketHeader,
                PacketHeader::new(
                    $sslot.req_type,
                    $resp.len() as _,
                    $sess.peer_sess_id,
                    $sslot.req_idx,
                    if likely($resp.is_small()) {
                        PktType::SmallResp
                    } else {
                        PktType::LargeResp
                    }
                ),
            )
        };

        // Store the response buffer in the SSlot.
        // Previous response buffer will be buried at this point.
        $sslot.resp = $resp;
        $sslot.finished = true;

        // Push the packet to the pending TX queue.
        $state.pending_tx.push(TxItem {
            peer: $sess.peer.as_ref().unwrap(),
            pkthdr: &$sslot.pkthdr,
            msgbuf: &$sslot.resp,
        });
    };

    (RETRANSMIT, $id:expr, $state:ident, $sess:ident, $sslot:ident, $req_idx:ident, $req_type:ident) => {
        // Simply drop outdated requests.
        if $req_idx < $sslot.req_idx {
            log::debug!(
                "RPC {}: dropping received SmallReq for outdated request (idx {}, ty {})",
                $id,
                $req_idx,
                $req_type
            );
        } else {
            // Here, this must be a retransmitted request.
            // RPCs should finish in a short time, so likely finished but packet lost.
            if likely($sslot.finished) {
                // If the request is already finished, we need to retransmit the response.
                log::debug!(
                    "RPC {}: retransmitting possibly-lost response for request (idx {}, ty {})",
                    $id,
                    $req_idx,
                    $req_type
                );

                // Setup retransmission item.
                $state.pending_tx.push(TxItem {
                    peer: $sess.peer.as_ref().unwrap(),
                    pkthdr: &$sslot.pkthdr,
                    msgbuf: &$sslot.resp,
                });

                // Need to drain the DMA queue.
                // This is because a previous response might have already reached the client,
                // i.e., this is an actually unnecessary retransmission. But if a future
                // request is sent before this retransmission, the response can get corrupted.
                //
                // SAFETY: `item` points to a valid peer and `MsgBuf`.
                $state.drain_tx_batch(true);
            } else {
                // If the request is simply not finished, we have nothing to do.
                // Let the client wait.
                log::debug!(
                    "RPC {}: client urging for response of request (idx {}, ty {}), but it is not finished yet",
                    $id,
                    $req_idx,
                    $req_type
                );
            }
        }
    };
}

// Internal progress.Rx routines.
impl Rpc {
    /// Process a single-packet incoming request.
    /// Return `true` if the request handler returns immediately or not even called
    /// (which means the caller can release the Rx buffer).
    ///
    /// # Panics
    ///
    /// Panic if `self.state` is already borrowed.
    #[must_use = "you must the return value to decide whether to release the Rx buffer"]
    fn process_small_request(
        &self,
        sess_id: SessId,
        sslot_idx: usize,
        pkthdr: &PacketHeader,
        req_msgbuf: &MsgBuf,
    ) -> bool {
        // SAFETY: guaranteed not null and aligned.
        debug_assert_eq!(
            pkthdr.pkt_type(),
            PktType::SmallReq,
            "packet type is not small-request"
        );

        // Check request type sanity.
        let req_type = pkthdr.req_type();
        if unlikely(!self.has_rpc_handler(req_type)) {
            log::debug!(
                "RPC {}: dropping received SmallReq for unknown request type {:?}",
                self.id,
                req_type
            );
            return true;
        }

        let mut real_state = self.state.borrow_mut();
        let state: &mut RpcInterior = &mut real_state;
        let sess = &mut state.sessions[sess_id as usize];
        let sslot = &mut sess.slots[sslot_idx];

        // Check request index sanity.
        let req_idx = pkthdr.req_idx();
        if unlikely(req_idx <= sslot.req_idx) {
            do_response_tx!(RETRANSMIT, self.id, state, sess, sslot, req_idx, req_type);
            return true;
        }

        // Now that this must be a valid *new* request.
        // Fill in the SSlot fields.
        sslot.finished = false;
        sslot.req_idx = req_idx;
        sslot.req_type = req_type;

        // Use `MsgBuf::clone_borrowed()` to keep the lkey, which is needed when releasing the buffer.
        // This will bury the previous request buffer if it was a large message.
        sslot.req = req_msgbuf.clone_borrowed();
        sslot.req_borrowed = true;

        // Construct the request. This will fetch the raw pointer of the current `Rpc` and `sslot`.
        let request = RequestHandle::new(self, sslot);
        drop(real_state);

        // Call the request handler with no `state` borrows.
        let mut resp_fut = self.call_rpc_handler(request);

        // Immediately poll the Future.
        // This will make synchronous handlers return, and push asynchronous handlers
        // into the first yield point.
        let mut cx = Context::from_waker(noop_waker_ref());
        let response = resp_fut.as_mut().poll(&mut cx);

        // Now we can borrow `state` again.
        let mut real_state = self.state.borrow_mut();
        let state: &mut RpcInterior = &mut real_state;
        let sess = &mut state.sessions[sess_id as usize];
        let sslot = &mut sess.slots[sslot_idx];
        match response {
            Poll::Ready(resp) => {
                assert!(
                    !sslot.has_handle,
                    "RequestHandle not dropped after handler finishes"
                );
                do_response_tx!(ENQUEUE, state, sess, sslot, resp);
                true
            }
            Poll::Pending => {
                state
                    .pending_handlers
                    .push(PendingHandler::new(sess_id, sslot_idx, resp_fut));
                false
            }
        }
    }

    /// Process a control message of a large request.
    /// Always release the corresponding transport-held buffer when returning.
    ///
    /// This method never calls request handlers; instead, it only updates the `SSlot`
    /// and posts an RDMA read to fetch the request data. Therefore, it is safe to take
    /// a `&mut RpcInterior` as argument.
    fn process_large_request_control(
        &self,
        state: &mut RpcInterior,
        sess_id: SessId,
        sslot_idx: usize,
        hdr: &PacketHeader,
        ctrl: &ControlMsg,
    ) {
        debug_assert_eq!(
            hdr.pkt_type(),
            PktType::LargeReq,
            "packet type is not large-request-ctrl"
        );

        // Check request type sanity.
        let req_type = hdr.req_type();
        if unlikely(!self.has_rpc_handler(req_type)) {
            log::debug!(
                "RPC {}: dropping received LargeReq for unknown request type {:?}",
                self.id,
                req_type
            );
            return;
        }

        let sess = &mut state.sessions[sess_id as usize];
        let sslot = &mut sess.slots[sslot_idx];

        // Check request index sanity.
        let req_idx = hdr.req_idx();
        if unlikely(req_idx <= sslot.req_idx) {
            do_response_tx!(RETRANSMIT, self.id, state, sess, sslot, req_idx, req_type);
            return;
        }

        // Fill in the SSlot fields.
        sslot.finished = false;
        sslot.req_idx = hdr.req_idx();
        sslot.req_type = req_type;

        // Prepare a buffer for the request.
        let data_len = hdr.data_len() as usize;
        let req_buf = state.buddy.alloc(data_len, &mut state.tp);
        sslot.req = MsgBuf::owned_immutable(req_buf, data_len);
        sslot.req_borrowed = false;

        // Downgrade the borrow to SSlot as immutable to make the borrow checker happy.
        let sslot = &sess.slots[sslot_idx];

        // Post an RDMA read to fetch the request data.
        state
            .rc_tp
            .post_rc_read(sess_id, sslot_idx, &sess.rc_qp, &sslot.req, ctrl);
    }

    /// Process an actual large request.
    /// This method need not handle any retransmission logic, as that is already
    /// done by the control buffer handler ([`process_large_request_control()`]).
    ///
    /// # Panics
    ///
    /// Panic if `self.state` is already borrowed.
    #[allow(dropping_references)]
    fn process_large_request_body(&self, sess_id: SessId, sslot_idx: usize) {
        let mut state = self.state.borrow_mut();
        let sess = &mut state.sessions[sess_id as usize];
        let sslot = &mut sess.slots[sslot_idx];

        // No need for any sanity check, as that is already done when handling the control message.
        // Construct the request. This will fetch the raw pointer of the current `Rpc` and `sslot`.
        let request = RequestHandle::new(self, sslot);
        drop(state);

        // Call the request handler with no `state` borrows.
        let mut resp_fut = self.call_rpc_handler(request);

        // Immediately poll the Future.
        // This will make synchronous handlers return, and push asynchronous handlers
        // into the first yield point.
        let mut cx = Context::from_waker(noop_waker_ref());
        let response = resp_fut.as_mut().poll(&mut cx);

        // Now we can borrow `state` again.
        let mut real_state = self.state.borrow_mut();
        let state: &mut RpcInterior = &mut real_state;
        let sess = &mut state.sessions[sess_id as usize];
        let sslot = &mut sess.slots[sslot_idx];
        match response {
            Poll::Ready(resp) => {
                assert!(
                    !sslot.has_handle,
                    "RequestHandle not dropped after handler finishes"
                );
                do_response_tx!(ENQUEUE, state, sess, sslot, resp);
            }
            Poll::Pending => {
                state
                    .pending_handlers
                    .push(PendingHandler::new(sess_id, sslot_idx, resp_fut));
            }
        }
    }

    /// Process a single-packet response.
    /// Always release the correponding transport-held buffer when returning.
    ///
    /// This method never calls request handlers, so it is safe to take a `&mut RpcInterior`
    /// as argument.
    fn process_small_response(
        &self,
        state: &mut RpcInterior,
        sess_id: SessId,
        sslot_idx: usize,
        hdr: &PacketHeader,
        data: *const u8,
    ) {
        debug_assert_eq!(
            hdr.pkt_type(),
            PktType::SmallResp,
            "packet type is not small-response"
        );

        let sess = &mut state.sessions[sess_id as usize];
        let sslot = &mut sess.slots[sslot_idx];

        // Check packet metadata sanity.
        if unlikely(hdr.req_idx() != sslot.req_idx) {
            assert!(
                hdr.req_idx() < sslot.req_idx,
                "response for future request is impossible: received {}, current {}",
                hdr.req_idx(),
                sslot.req_idx
            );
            log::debug!(
                "RPC {}: dropping received SmallResponse for expired request (idx {}, current {})",
                self.id,
                hdr.req_idx(),
                sslot.req_idx
            );
            return;
        }

        // Truncate response if needed.
        let len = cmp::min(sslot.resp.capacity(), hdr.data_len() as usize);
        sslot.resp.set_len(len);

        // SAFETY: source is guaranteed to be valid, destination length checked, may not overlap.
        unsafe { ptr::copy_nonoverlapping(data, sslot.resp.as_mut_ptr(), len) };
        sslot.finished = true;
    }

    /// Process a control message of a large response.
    /// Always release the corresponding transport-held buffer when returning.
    ///
    /// This method never calls request handlers, so it is safe to take a `&mut RpcInterior`
    /// as argument.
    fn process_large_response_control(
        &self,
        state: &mut RpcInterior,
        sess_id: SessId,
        sslot_idx: usize,
        hdr: &PacketHeader,
        ctrl: &ControlMsg,
    ) {
        debug_assert_eq!(
            hdr.pkt_type(),
            PktType::LargeResp,
            "packet type is not large-response-ctrl"
        );

        let sess = &mut state.sessions[sess_id as usize];
        let sslot = &mut sess.slots[sslot_idx];

        // Check packet metadata sanity.
        if unlikely(hdr.req_idx() != sslot.req_idx) {
            assert!(
                hdr.req_idx() < sslot.req_idx,
                "response for future request is impossible: received {}, current {}",
                hdr.req_idx(),
                sslot.req_idx
            );
            log::debug!(
                "RPC {}: dropping received LargeResponse for expired request (idx {}, current {})",
                self.id,
                hdr.req_idx(),
                sslot.req_idx
            );
            return;
        }

        // Truncate response if needed.
        let len = cmp::min(sslot.resp.capacity(), hdr.data_len() as usize);
        sslot.resp.set_len(len);

        // Post an RDMA read to fetch the response data.
        state
            .rc_tp
            .post_rc_read(sess_id, sslot_idx, &sess.rc_qp, &sslot.resp, ctrl);
    }

    /// Process an actual large response.
    ///
    /// This method never calls request handlers, so it is safe to take a `&mut Session`
    /// as argument (derived from interior state).
    #[inline]
    fn process_large_response_body(&self, sess: &mut Session, sslot_idx: usize) {
        // Data already in the buffer, just mark the request as finished.
        sess.slots[sslot_idx].finished = true;
    }
}

// Internal progress routines.
impl Rpc {
    /// Process received session management events.
    /// This method works purely on the control plane.
    fn process_sm_events(&self) {
        // Should not panic, as we have checked reentrancy in `progress()`.
        let mut real_state = self.state.borrow_mut();
        let state: &mut RpcInterior = &mut real_state;

        while let Some(event) = self.sm_rx.recv() {
            log::trace!("RPC {}: received {:#?}", self.id, event);
            debug_assert_eq!(event.dst_rpc_id, self.id, "bad SM event dispatch");

            match event.details {
                SmEventDetails::ConnectRequest {
                    cli_uri: uri,
                    cli_ud_ep,
                    cli_sess_id,
                    cli_sess_rc_ep,
                } => {
                    // Refuse connection if there are too many sessions.
                    if unlikely(state.sessions.len() > SessId::MAX as usize) {
                        let nak = SmEvent {
                            src_rpc_id: self.id,
                            dst_rpc_id: event.src_rpc_id,
                            details: SmEventDetails::ConnectRefuse {
                                cli_sess_id,
                                reason: ConnectRefuseReason::SessionLimitExceeded,
                            },
                        };
                        let nak_buf = rmps::to_vec(&nak).unwrap();
                        self.sm_tx
                            .send_to(&nak_buf, uri)
                            .expect("failed to send ConnectRefuse");
                        continue;
                    }

                    let peer = state.tp.create_peer(cli_ud_ep);
                    let svr_ud_ep = state.tp.endpoint();

                    // Setup an initially connected session.
                    // First, create an RC QP for it.
                    let mut rc_qp = state.rc_tp.create_qp(&state.tp);
                    rc_qp
                        .bind_peer(cli_sess_rc_ep)
                        .expect("failed to bind server-side RC QP to peer");
                    let svr_sess_rc_ep = rc_qp.endpoint().unwrap();

                    // Then, initialize the session.
                    let mut sess = Session::new(state, SessionRole::Server, rc_qp);
                    sess.peer_uri = uri;
                    sess.peer_rpc_id = event.src_rpc_id;
                    sess.peer_sess_id = cli_sess_id;
                    sess.peer = Some(peer);
                    sess.connected = Some(true);

                    let svr_sess_id = state.sessions.len() as SessId;
                    state.sessions.push(sess);

                    // Send ConnectAcknowledge.
                    let ack = SmEvent {
                        src_rpc_id: self.id,
                        dst_rpc_id: event.src_rpc_id,
                        details: SmEventDetails::ConnectAcknowledge {
                            cli_sess_id,
                            svr_ud_ep,
                            svr_sess_id,
                            svr_sess_rc_ep,
                        },
                    };
                    let ack_buf = rmps::to_vec(&ack).unwrap();
                    self.sm_tx
                        .send_to(&ack_buf, uri)
                        .expect("failed to send ConnectAcknowledge");
                }
                SmEventDetails::ConnectAcknowledge {
                    cli_sess_id,
                    svr_ud_ep,
                    svr_sess_id,
                    svr_sess_rc_ep,
                } => {
                    if unlikely(state.sessions.len() <= cli_sess_id as usize) {
                        log::debug!(
                            "RPC {}: ignoring ConnectAcknowledge for non-existent session {}",
                            self.id,
                            cli_sess_id
                        );
                        continue;
                    }

                    // Check session sanity.
                    let sess = &mut state.sessions[cli_sess_id as usize];
                    if unlikely(
                        sess.is_connected()
                            || !sess.is_client()
                            || sess.peer_rpc_id != event.src_rpc_id,
                    ) {
                        log::debug!(
                            "RPC {}: ignoring ConnectAcknowledge for client session {}, because of (connected, role, peer_rpc_id) mismatch: expected {:?}, found {:?}",
                            self.id,
                            cli_sess_id,
                            (false, SessionRole::Client, sess.peer_rpc_id),
                            (sess.is_connected(), SessionRole::Server, event.src_rpc_id),
                        );
                        continue;
                    }

                    // Fill in session fields ...
                    let peer = state.tp.create_peer(svr_ud_ep);
                    sess.peer_sess_id = svr_sess_id;
                    sess.peer = Some(peer);
                    sess.connected = Some(true);

                    // ... and connect the RC QP to the remote.
                    sess.rc_qp
                        .bind_peer(svr_sess_rc_ep)
                        .expect("failed to bind client-side RC QP to peer");
                }
                SmEventDetails::ConnectRefuse {
                    cli_sess_id,
                    reason,
                } => {
                    if state.sessions.len() <= cli_sess_id as usize {
                        log::debug!(
                            "RPC {}: ignoring ConnectRefuse for non-existent session {}",
                            self.id,
                            cli_sess_id
                        );
                        continue;
                    }

                    log::debug!(
                        "RPC {}: session {} is refused by remote peer, due to {:?}",
                        self.id,
                        cli_sess_id,
                        reason
                    );
                    state.sessions[event.dst_rpc_id as usize].peer.take();
                }
                SmEventDetails::Disconnect => unimplemented!(),
            }
        }
    }

    /// Process received datapath packets.
    fn process_rx(&self) {
        // Should not panic, as we have checked reentrancy in `progress()`.
        let mut real_state = self.state.borrow_mut();
        let state: &mut RpcInterior = &mut real_state;

        // First, do UD Rx burst.
        let n = state.tp.rx_burst();

        // Process the received packets.
        let mut rx_bufs_to_release = Vec::with_capacity(n);

        // Perform sanity check, and handle received responses.
        // Requests are handled later since they will call request handler functions
        // and require `state` unborrowed.
        let mut rx_requests = Vec::with_capacity(n);
        for _ in 0..n {
            let item = state.tp.rx_next().expect("failed to fetch received packet");

            // SAFETY: this is a transport receive buffer, so we can call `pkt_hdr()` to get the header.
            // Pointers are guaranteed not null and aligned.
            let hdr = unsafe { NonNull::new_unchecked(item.pkt_hdr()).as_ref() };
            let sess_id = hdr.dst_sess_id();
            if unlikely(state.sessions.len() <= sess_id as usize) {
                log::debug!(
                    "RPC {}: dropping received data-plane packet for non-existent session {}",
                    self.id,
                    sess_id
                );
                continue;
            }

            // Perform session sanity check.
            let sess = &state.sessions[sess_id as usize];
            if unlikely(!sess.is_connected()) {
                log::debug!(
                    "RPC {}: dropping received data-plane packet for non-connected session {}",
                    self.id,
                    sess_id
                );
                continue;
            }

            let pkt_type = hdr.pkt_type();
            match pkt_type {
                PktType::SmallReq | PktType::LargeReq => {
                    if unlikely(sess.is_client()) {
                        log::debug!(
                            "RPC {}: dropping received {:?} for client session {}",
                            self.id,
                            pkt_type,
                            sess_id
                        );
                        continue;
                    }
                }
                PktType::SmallResp | PktType::LargeResp => {
                    if unlikely(!sess.is_client()) {
                        log::debug!(
                            "RPC {}: dropping received {:?} for server session {}",
                            self.id,
                            pkt_type,
                            sess_id
                        );
                        continue;
                    }
                }
            }

            // Perform packet sanity check.
            if matches!(pkt_type, PktType::SmallReq | PktType::SmallResp)
                && unlikely(hdr.data_len() > UdTransport::max_data_in_pkt() as u32)
            {
                log::debug!(
                    "RPC {}: dropping received {:?} with too large data length {}",
                    self.id,
                    pkt_type,
                    hdr.data_len()
                );
                continue;
            }

            if unlikely(
                matches!(pkt_type, PktType::LargeReq | PktType::LargeResp)
                    && item.len() != mem::size_of::<ControlMsg>(),
            ) {
                log::debug!(
                    "RPC {}: dropping received {:?} with invalid control packet length {}",
                    self.id,
                    pkt_type,
                    item.len()
                );
                continue;
            }

            // Trigger packet processing logic.
            let sslot_idx = hdr.req_idx() as usize % ACTIVE_REQ_WINDOW;
            match pkt_type {
                PktType::SmallReq => rx_requests.push(item),
                PktType::LargeReq => {
                    // SAFETY: control packet sanity checked.
                    let ctrl = unsafe { &*(item.as_ptr() as *const ControlMsg) };
                    self.process_large_request_control(state, sess_id, sslot_idx, hdr, ctrl);
                    rx_bufs_to_release.push(item);
                }
                PktType::SmallResp => {
                    self.process_small_response(state, sess_id, sslot_idx, hdr, item.as_ptr());
                    rx_bufs_to_release.push(item);
                }
                PktType::LargeResp => {
                    // SAFETY: control packet sanity checked.
                    let ctrl = unsafe { &*(item.as_ptr() as *const ControlMsg) };
                    self.process_large_response_control(state, sess_id, sslot_idx, hdr, ctrl);
                    rx_bufs_to_release.push(item);
                }
            }
        }

        // Second, do RC Rx burst.
        // This step processes only self-produced data, so no need for release-mode
        // runtime sanity check.
        let n = state.rc_tp.tx_completion_burst();
        let mut rx_large_requests = Vec::with_capacity(n);

        for (sess_id, sslot_idx) in state.rc_tp.tx_done() {
            debug_assert!((sess_id as usize) < state.sessions.len());
            debug_assert!(sslot_idx < ACTIVE_REQ_WINDOW);

            let sess = &mut state.sessions[sess_id as usize];
            if sess.is_client() {
                self.process_large_response_body(sess, sslot_idx);
            } else {
                rx_large_requests.push((sess_id, sslot_idx));
            }
        }

        // Drop the mutable borrow ...
        drop(real_state);

        // ... so that we can start to handle requests.
        for item in rx_requests {
            // SAFETY: this is a transport receive buffer, so we can call `pkt_hdr()` to get the header.
            // Pointers are guaranteed not null and aligned.
            let hdr = unsafe { NonNull::new_unchecked(item.pkt_hdr()).as_ref() };
            let sess_id = hdr.dst_sess_id();
            let sslot_idx = hdr.req_idx() as usize % ACTIVE_REQ_WINDOW;

            if self.process_small_request(sess_id, sslot_idx, hdr, &item) {
                rx_bufs_to_release.push(item);
            }
        }

        for (sess_id, sslot_idx) in rx_large_requests {
            self.process_large_request_body(sess_id, sslot_idx);
        }

        // Release the Rx buffers to the transport layer.
        // SAFETY: `rx_bufs_to_release` contains valid Rx buffers, and each buffer
        // is only released once (which is this release).
        let mut real_state = self.state.borrow_mut();
        for item in rx_bufs_to_release {
            unsafe { real_state.tp.rx_release(&item) };
        }
    }

    /// Transmit pending packets.
    fn process_tx(&self) {
        // Should not panic, as we have checked reentrancy in `progress()`.
        let mut state = self.state.borrow_mut();
        state.drain_tx_batch(false);
    }

    /// Poll pending request handlers.
    fn process_pending_handlers(&self) {
        // Should not panic, as we have checked reentrancy in `progress()`.
        let mut real_state = self.state.borrow_mut();
        let state: &mut RpcInterior = &mut real_state;

        // Fetch the pending handlers into a local context.
        // This is to prevent borrowing `state` while we are polling the handlers.
        let mut pending_handlers = Vec::new();
        mem::swap(&mut state.pending_handlers, &mut pending_handlers);

        // Drop the mutable borrow ...
        drop(real_state);

        // ... so that we can start to poll the handlers.
        let mut finished_handlers = Vec::with_capacity(pending_handlers.len());
        let mut cx = Context::from_waker(noop_waker_ref());
        pending_handlers.retain_mut(|resp_fut| match resp_fut.handler.as_mut().poll(&mut cx) {
            Poll::Ready(resp) => {
                finished_handlers.push((resp_fut.sess_id, resp_fut.sslot_idx, resp));
                false
            }
            Poll::Pending => true,
        });

        // Now we can borrow `state` again.
        let mut real_state = self.state.borrow_mut();
        let state: &mut RpcInterior = &mut real_state;

        // First, put the pending handlers back to `state`.
        debug_assert!(
            state.pending_handlers.is_empty(),
            "there should be no reentrant `progress()`, why are there new pending handlers?"
        );
        mem::swap(&mut state.pending_handlers, &mut pending_handlers);

        // Then, transmit the responses.
        for (sess_id, sslot_idx, resp) in finished_handlers {
            let sess = &mut state.sessions[sess_id as usize];
            let sslot = &mut sess.slots[sslot_idx];

            assert!(
                !sslot.has_handle,
                "RequestHandle not dropped after handler finishes"
            );

            // Release the request buffer, but only if it is from the transport layer.
            // We identify this from the `aux_data` field, which is set to `(1 << 32) | idx` for
            // receive buffers from the UD transport.
            if sslot.req_borrowed {
                // SAFETY: this buffer is not released before because:
                // - `process_small_request()` returns `false`, preventing it from getting
                //   released in `process_rx()`;
                // - previous polls to the handler future gives `Pending`, not doing anything
                //   to this buffer.
                unsafe { state.tp.rx_release(&sslot.req) };
            }

            do_response_tx!(ENQUEUE, state, sess, sslot, resp);
        }
    }
}

/// A macro that prepares a `SSlot` for the next request.
///
/// This is a macro instead of a method becuase the latter will make the borrow
/// checker very unhappy.
macro_rules! do_sslot_move_next {
    ($state:ident, $sess:ident, $sslot:ident, $sslot_idx:ident) => {
        // Move the SSlot to  the next request.
        $sslot.req_idx += ACTIVE_REQ_WINDOW as ReqIdx;
        $sslot.finished = false;

        // Find the next pending request, if there are any.
        let pending_req = if unlikely(!$sess.req_backlog[$sslot_idx].is_empty()) {
            let mut ret = None;
            while let Some(req) = $sess.req_backlog[$sslot_idx].pop_front() {
                if likely(!req.aborted) {
                    ret = Some(req);
                    break;
                }
                $sslot.req_idx += ACTIVE_REQ_WINDOW as ReqIdx;
            }
            ret
        } else {
            None
        };

        // If there exist a pending request, reuse the `SSlot` and start transmission.
        if let Some(pending_req) = pending_req {
            debug_assert!(
                pending_req.expected_req_idx == $sslot.req_idx,
                "mismatch req_idx for pending request: expected {}, got {}",
                pending_req.expected_req_idx,
                $sslot.req_idx
            );
            $sslot.req = pending_req.req;
            $sslot.resp = pending_req.resp;

            // Fill in the request packet header.
            // SAFETY: packet header validity checked on creation.
            unsafe {
                ptr::copy_nonoverlapping(
                    &pending_req.pkthdr,
                    $sslot.pkthdr.as_ptr() as *mut PacketHeader,
                    1,
                )
            };

            // Push the packet into the Tx queue.
            $state.pending_tx.push(TxItem {
                peer: $sess.peer.as_ref().unwrap(),
                pkthdr: &$sslot.pkthdr,
                msgbuf: &$sslot.req,
            });
        } else {
            // Free the SSlot.
            $sess.avail_slots.push_back($sslot_idx);
        }
    };
}

// Crate-internal APIs.
impl Rpc {
    /// Return `true` if the given request type is registered with a request handler.
    #[inline(always)]
    pub(crate) fn has_rpc_handler(&self, req_type: ReqType) -> bool {
        self.handlers[req_type as usize].is_some()
    }

    /// Call the registered request handler for the given request type.
    ///
    /// # Panics
    ///
    /// Panic if there is no such request handler.
    #[inline]
    pub(crate) fn call_rpc_handler(&self, req: RequestHandle) -> ReqHandlerFuture {
        let req_type = req.req_type();
        let handler = self.handlers[req_type as usize].as_ref().unwrap();
        handler(req)
    }

    /// Return `true` if the session of the given ID exists and is connected.
    ///
    /// # Panics
    ///
    /// - Panic if the session ID is invalid.
    /// - Panic if called when `self.state` is already borrowed.
    #[inline]
    pub(crate) fn session_connection_state(&self, sess_id: SessId) -> Option<bool> {
        self.state.borrow().sessions[sess_id as usize].connected
    }

    /// Mark a session as connecting in progress.
    /// Return the UD endpoint and the RC endpoint for connection use.
    ///
    /// # Panics
    ///
    /// - Panic if the session ID is invalid.
    /// - Panic if the session is already connected.
    /// - Panic if called when `self.state` is already borrowed.
    #[inline]
    pub(crate) fn mark_session_connecting(&self, sess_id: SessId) -> (QpEndpoint, QpEndpoint) {
        let mut real_state = self.state.borrow_mut();
        let state: &mut RpcInterior = &mut real_state;

        // Mark the session as connecting.
        let sess = &mut state.sessions[sess_id as usize];
        if let Some(true) = sess.connected {
            panic!("session {} is already connected", sess_id);
        }
        sess.connected = None;

        // Return the endpoints: (UD endpoint, RC endpoint).
        (state.tp.endpoint(), sess.rc_qp.endpoint().unwrap())
    }

    /// Enqueue a request for a session.
    /// Return the `SSlot` index assigned to this request.
    ///
    /// # Panics
    ///
    /// - Panic if the session ID is invalid.
    /// - Panic if called when `self.state` is already borrowed.
    pub(crate) fn enqueue_request<'a>(
        &'a self,
        sess_id: SessId,
        req_type: ReqType,
        req_msgbuf: &'a MsgBuf,
        resp_msgbuf: &'a mut MsgBuf,
    ) -> Request<'a> {
        let mut real_state = self.state.borrow_mut();
        let state: &mut RpcInterior = &mut real_state;

        let sess = &mut state.sessions[sess_id as usize];

        // Fetch an available slot, enqueue to the shortest queue if failing.
        if let Some(sslot_idx) = sess.avail_slots.pop_front() {
            // Fill in SSlot entries.
            let sslot = &mut sess.slots[sslot_idx];
            sslot.finished = false;
            sslot.req = req_msgbuf.clone_borrowed();
            sslot.resp = resp_msgbuf.clone_borrowed();

            // Fill in the request packet header.
            // SAFETY: packet header validity checked on creation.
            unsafe {
                ptr::write_volatile(
                    sslot.pkthdr.as_ptr() as *mut PacketHeader,
                    PacketHeader::new(
                        req_type,
                        req_msgbuf.len() as _,
                        sess.peer_sess_id,
                        sslot.req_idx,
                        if likely(req_msgbuf.is_small()) {
                            PktType::SmallReq
                        } else {
                            PktType::LargeReq
                        },
                    ),
                )
            };

            // Push the request into the Tx queue.
            state.pending_tx.push(TxItem {
                peer: sess.peer.as_ref().unwrap(),
                pkthdr: &sslot.pkthdr,
                msgbuf: &sslot.req,
            });

            Request::new(self, resp_msgbuf, sess_id, sslot_idx, sslot.req_idx)
        } else {
            // Find the shortest backlog queue.
            let (sslot_idx, pending_len) = sess
                .req_backlog
                .iter()
                .enumerate()
                .map(|(i, queue)| (i, queue.len()))
                .min_by_key(|(_, len)| *len)
                .unwrap();

            // Invariant: if there are pending requests, then the SSlot must be non-empty.
            // So, request index is current index + (existing backlog length + 1) * ACTIVE_REQ_WINDOW.
            let req_idx =
                sess.slots[sslot_idx].req_idx + ((pending_len + 1) * ACTIVE_REQ_WINDOW) as ReqIdx;

            // Generate the packet header.
            let pkthdr = PacketHeader::new(
                req_type,
                req_msgbuf.len() as _,
                sess.peer_sess_id,
                req_idx,
                if likely(req_msgbuf.is_small()) {
                    PktType::SmallReq
                } else {
                    PktType::LargeReq
                },
            );

            // Push the request into the pending queue.
            sess.req_backlog[sslot_idx].push_back(PendingRequest {
                pkthdr,
                req: req_msgbuf.clone_borrowed(),
                resp: resp_msgbuf.clone_borrowed(),
                expected_req_idx: req_idx,
                aborted: false,
            });

            Request::new(self, resp_msgbuf, sess_id, sslot_idx, req_idx)
        }
    }

    /// Check and handle the completion status of a request.
    ///
    /// Upon completion, modify the passed-in response buffer to the correct size.
    /// Also, update the `SSlot` to mark the current request as outdated.
    /// If there is a waiting request, make it active.
    /// Otherwise, if the request is unfinished and the caller asks for retransmission,
    /// add the retransmission item to the pending Tx queue.
    ///
    /// For now, we use this function to finalize a request, instead of doing so in
    /// [`Self::process_rx()`]. Possibly fix this in the future.
    ///
    /// # Panics
    ///
    /// - Panic if the session ID or SSlot index is invalid.
    /// - Panic if called when `self.state` is already borrowed.
    pub(crate) fn check_request_completion(
        &self,
        (sess_id, sslot_idx, req_idx): (SessId, usize, ReqIdx),
        resp_buf: &mut MsgBuf,
        re_tx: bool,
    ) -> bool {
        let mut real_state = self.state.borrow_mut();
        let state: &mut RpcInterior = &mut real_state;

        let sess = &mut state.sessions[sess_id as usize];
        let sslot = &mut sess.slots[sslot_idx];

        assert_eq!(
            sslot.req_idx as usize % ACTIVE_REQ_WINDOW,
            req_idx as usize % ACTIVE_REQ_WINDOW
        );

        // Request has already expired, so it must be finished.
        // Would this really happen?
        if unlikely(sslot.req_idx > req_idx) {
            return true;
        }

        // Request is still in the pending queue, so it must be unfinished.
        if sslot.req_idx < req_idx {
            return false;
        }

        // Now we must have `sslot.req_idx == req_idx`.
        if sslot.finished {
            // Modify user-provided response buffer length.
            resp_buf.set_len(sslot.resp.len());

            do_sslot_move_next!(state, sess, sslot, sslot_idx);
            return true;
        }

        // Not finished, handle retransmission.
        if unlikely(re_tx) {
            state.pending_tx.push(TxItem {
                peer: sess.peer.as_ref().unwrap(),
                pkthdr: &sslot.pkthdr,
                msgbuf: &sslot.req,
            });
        }
        false
    }

    /// Abort a request.
    pub(crate) fn abort_request(&self, (sess_id, sslot_idx, req_idx): (SessId, usize, ReqIdx)) {
        let mut real_state = self.state.borrow_mut();
        let state: &mut RpcInterior = &mut real_state;

        let sess = &mut state.sessions[sess_id as usize];
        let sslot = &mut sess.slots[sslot_idx];

        // Request has already expired, so aborting it is no-op.
        if unlikely(sslot.req_idx > req_idx) {
            return;
        }

        if likely(sslot.req_idx == req_idx) {
            // Request is in `SSlot`, similar to request completion.
            do_sslot_move_next!(state, sess, sslot, sslot_idx);
        } else {
            // Request is in the pending queue, make it aborted.
            let pending_req = sess.req_backlog[sslot_idx]
                .iter_mut()
                .find(|pending_req| pending_req.expected_req_idx == req_idx)
                .unwrap();
            pending_req.aborted = true;
        }
    }
}

// Public APIs.
impl Rpc {
    /// Create a new `Rpc` instance that is bound to a [`Nexus`] with a certain
    /// ID and the current thread. Will operate on the specified port of the given
    /// device. The given ID must be unique among all RPCs in the same Nexus.
    ///
    /// The created `Rpc` instance is pinned on heap to prevent moving around.
    /// Otherwise, it can invalidate pointers recorded in request handles.
    ///
    /// # Panics
    ///
    /// - Panic if the given ID is already used.
    /// - Panic if there is no such device or no such port.
    pub fn new(nexus: &Arc<Nexus>, id: RpcId, nic: &str, phy_port: u8) -> Pin<Box<Self>> {
        const PREALLOC_SIZE: usize = 64;

        // Create the SM event channel first, so that it will immediately
        // panic if the given ID is already used.
        let sm_rx = nexus.register_event_channel(id);
        let tp = UdTransport::new(nic, phy_port);
        Box::pin(Self {
            id,
            nexus: nexus.clone(),
            sm_tx: UdpSocket::bind("0.0.0.0:0").unwrap(),
            sm_rx,
            state: InteriorCell::new(RpcInterior {
                sessions: Vec::new(),
                rc_tp: RcTransport::new(&tp),
                tp,
                pending_handlers: Vec::with_capacity(PREALLOC_SIZE),
                pending_tx: Vec::with_capacity(PREALLOC_SIZE),
                slab: SlabAllocator::new(),
                buddy: Rc::new(BuddyAllocator::new()),
            }),
            handlers: array::from_fn(|_| None),
            progressing: RefCell::new(()),
            _pinned: PhantomPinned,
        })
    }

    /// Return the ID of this `Rpc` instance.
    #[inline(always)]
    pub fn id(&self) -> RpcId {
        self.id
    }

    /// Return a reference to the `Nexus` bound to.
    #[inline(always)]
    pub fn nexus(&self) -> &Nexus {
        &self.nexus
    }

    /// Set the handler for the given request type.
    /// The handler takes a request handle as argument, and must return a `MsgBuf` that
    /// belongs to the same `Rpc` calling the handler (otherwise, panic).
    ///
    /// # Panics
    ///
    /// Panic if there is already an `Rpc` created.
    pub fn set_handler<H, F>(self: &mut Pin<Box<Self>>, req_id: ReqType, handler: H)
    where
        H: Fn(RequestHandle) -> F + 'static,
        F: Future<Output = MsgBuf> + 'static,
    {
        // SAFETY: `self` is treated as pinned in this method.
        let this = unsafe { Pin::into_inner_unchecked(self.as_mut()) };
        this.handlers[req_id as usize] = Some(Box::new(move |req| Box::pin(handler(req))));
    }

    /// Create a client session that connects to a remote `Rpc` instance.
    ///
    /// The created session is not connected by default, and it will spend no
    /// efforts on establishing the connection until you call
    /// [`Session::connect()`](SessionHandle::connect) on it.
    ///
    /// The returned object does not have ownership of the session, but is purely
    /// a handle to it. You may first remember the session ID and drop the handle.
    /// When needed, you can acquire a new handle by calling [`Rpc::get_session()`].
    pub fn create_session(
        &self,
        remote_uri: impl ToSocketAddrs,
        remote_rpc_id: RpcId,
    ) -> SessionHandle {
        // Get the URI first so that we can panic early.
        let remote_uri = remote_uri
            .to_socket_addrs()
            .expect("failed to resolve remote URI")
            .next()
            .expect("no such remote URI");

        // Borrow the state, thread-unsafe.
        let mut real_state = self.state.borrow_mut();
        let state: &mut RpcInterior = &mut real_state;

        assert!(
            state.sessions.len() < SessId::MAX as usize,
            "too many sessions"
        );

        // Create an RC QP for this session.
        let rc_qp = state.rc_tp.create_qp(&state.tp);

        // Initialize the session with remote peer information.
        // `peer` and `peer_sess_id` will be filled in when the connection is established.
        let sess_id = state.sessions.len() as SessId;
        let mut sess = Session::new(state, SessionRole::Client, rc_qp);
        sess.peer_uri = remote_uri;
        sess.peer_rpc_id = remote_rpc_id;
        state.sessions.push(sess);

        SessionHandle::new(self, sess_id, remote_uri, remote_rpc_id)
    }

    /// Return a handle to a session of the given ID.
    pub fn get_session(&self, sess_id: SessId) -> Option<SessionHandle> {
        // Borrow the state, thread-unsafe.
        let state = self.state.borrow();
        if state.sessions.len() <= sess_id as usize {
            return None;
        }

        let sess = &state.sessions[sess_id as usize];
        Some(SessionHandle::new(
            self,
            sess_id,
            sess.peer_uri,
            sess.peer_rpc_id,
        ))
    }

    /// Allocate a `MsgBuf` that can accommodate at least `len` bytes of
    /// application data. The allocated `MsgBuf` will have an initial length
    /// of `len`, but its content are uninitialized.
    ///
    /// `MsgBuf`s allocated by one `Rpc` instance can only be used in the same
    /// `Rpc` instance. This is because different `Rpc`s have different RDMA
    /// device contexts, and the `MsgBuf`'s lkey is bound to the device context.
    ///
    /// # Panics
    ///
    /// Panic if buffer allocation fails.
    #[inline]
    pub fn alloc_msgbuf(&self, len: usize) -> MsgBuf {
        self.state.borrow_mut().alloc_msgbuf(len)
    }

    /// Run an iteration of event loop to make progress.
    /// Performs tasks including:
    /// - handling session connection requests and responses,
    /// - hanlding datapath requests and responses,
    /// - scheduling and (re)transmitting datapath packets.
    #[inline]
    pub fn progress(&self) {
        // Prevent reentrance of this method.
        let Ok(_flag) = self.progressing.try_borrow_mut() else {
            return;
        };

        // Deal with session management events, which should be rare.
        if unlikely(!self.sm_rx.is_empty()) {
            self.process_sm_events();
        }

        // Ordering:
        // - Rx should be processed after polling pending handlers,
        //   or newcomers will be polled twice, the second time being
        //   largely meaningless.
        // - Tx should be processed after polling pending handlers and
        //   Rx, because they may generate response packets to be sent
        //   in `pending_tx`.
        self.process_pending_handlers();
        self.process_rx();
        self.process_tx();
    }
}

impl Drop for Rpc {
    fn drop(&mut self) {
        self.nexus.destroy_event_channel(self.id);
    }
}
