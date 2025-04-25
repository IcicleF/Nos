use std::{fmt, mem};

use rrddmma::{
    bindings::*,
    prelude::*,
    rdma::{
        cq::WcStatus,
        type_alias::{RKey, WrId},
    },
};

use super::ud::UdTransport;
use crate::{msgbuf::MsgBuf, type_alias::SessId};
use crate::{session::ACTIVE_REQ_WINDOW, util::likely::*};

/// Control packet payload for large messages.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct ControlMsg {
    /// The virtual address of the first byte of application data.
    pub addr: usize,

    /// The RKey of the memory region.
    pub rkey: RKey,
}

impl fmt::Debug for ControlMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ControlMsg")
            .field("addr", &format_args!("{:#x}", self.addr))
            .field("rkey", &format_args!("{:#x}", self.rkey))
            .finish()
    }
}

/// RDMA RC transport.
/// Processes large messages.
///
/// Since RC is connection-oriented, this type only stores common parts of the
/// RC QPs. QPs themselves are owned by the [`Session`](crate::session::Session)s.
pub(crate) struct RcTransport {
    /// The send completion queue.
    /// It is also the recv CQ, but this transport never posts receives.
    cq: Cq,
    /// Number of outstanding send requests.
    outstanding_sends: usize,
    /// Reserved work completions.
    wc: Vec<Wc>,
    /// `wr_id`s of finished sends.
    tx_done: Vec<WrId>,

    /// Cached RDMA read WR SGE.
    rdma_read_sge: Box<ibv_sge>,
    /// Cached RDMA read WR.
    rdma_read_wr: ibv_send_wr,
}

impl RcTransport {
    const CQ_DEPTH: usize = 256;
    const CQ_POLL_BATCH: usize = 16;
}

impl RcTransport {
    /// Create a new transport that bases itself on the given `UdTransport`.
    pub fn new(ud_tp: &UdTransport) -> Self {
        let ctx = ud_tp.pd().context();
        let cq = Cq::new(ctx, Self::CQ_DEPTH as _).expect("failed to create RC CQ");
        let wc = vec![Wc::default(); Self::CQ_POLL_BATCH];

        // Initialize the read SGE and WR.
        let mut rdma_read_sge = Box::<ibv_sge>::default();
        let rdma_read_wr = ibv_send_wr {
            sg_list: rdma_read_sge.as_mut(),
            num_sge: 1,
            opcode: ibv_wr_opcode::IBV_WR_RDMA_READ,
            send_flags: ibv_send_flags::IBV_SEND_SIGNALED.0,
            ..unsafe { mem::zeroed() }
        };

        Self {
            cq,
            outstanding_sends: 0,
            wc,
            tx_done: Vec::with_capacity(Self::CQ_DEPTH),

            rdma_read_sge,
            rdma_read_wr,
        }
    }

    /// Create a new RC QP on this transport instance.
    /// The returned QP is already bound to the specified local port.
    ///
    /// Due to inconviences in the crate `rrddmma`, we must pass the `UdTransport`
    /// again here.
    pub fn create_qp(&self, ud_tp: &UdTransport) -> Qp {
        let mut qp = Qp::builder()
            .qp_type(QpType::Rc)
            .send_cq(&self.cq)
            .recv_cq(&self.cq)
            .caps(QpCaps {
                max_send_wr: ACTIVE_REQ_WINDOW as _,
                max_recv_wr: 0,
                max_send_sge: 1,
                max_recv_sge: 0,
                ..Default::default()
            })
            .sq_sig_all(false)
            .build(ud_tp.pd())
            .expect("failed to create RC queue pair");
        qp.bind_local_port(ud_tp.port(), None)
            .expect("failed to bind RC QP to port");
        qp
    }

    /// Post an RDMA read request to the destination `MsgBuf`.
    #[inline]
    pub fn post_rc_read(
        &mut self,
        sess_id: SessId,
        sslot_idx: usize,
        rc_qp: &Qp,
        buf: &MsgBuf,
        ctrl: &ControlMsg,
    ) {
        // Reserve enough CQ space.
        while unlikely(self.outstanding_sends + 1 > Self::CQ_DEPTH) {
            self.tx_completion_burst();
        }
        self.outstanding_sends += 1;

        // Fill in the SGE & WR fields.
        *self.rdma_read_sge = ibv_sge {
            addr: buf.as_ptr() as _,
            length: buf.len() as _,
            lkey: buf.lkey(),
        };

        let wr = &mut self.rdma_read_wr;
        wr.wr_id = (sess_id as u64) << 32 | (sslot_idx as u64);
        wr.wr.rdma.remote_addr = ctrl.addr as _;
        wr.wr.rdma.rkey = ctrl.rkey;

        // SAFETY: the work request is correctly constructed.
        unsafe {
            rc_qp
                .post_raw_send(wr)
                .expect("failed to post read request");
        };
    }

    /// Poll the CQ to check for Tx completions.
    #[inline]
    pub fn tx_completion_burst(&mut self) -> usize {
        let n = self.cq.poll_into(&mut self.wc).expect("failed to poll CQ") as usize;
        assert!(n <= self.outstanding_sends);
        self.outstanding_sends -= n;

        for i in 0..n {
            assert_eq!(self.wc[i].status(), WcStatus::Success, "failed to read");
            self.tx_done.push(self.wc[i].wr_id());
        }
        self.tx_done.len()
    }

    /// Return the `wr_id`s of finished sends.
    #[inline]
    pub fn tx_done(&mut self) -> impl Iterator<Item = (SessId, usize)> + '_ {
        self.tx_done.drain(..).map(|wr_id| {
            let sess_id = (wr_id >> 32) as SessId;
            let sslot_idx = (wr_id as u8) as usize;
            (sess_id, sslot_idx)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_layout() {
        use std::mem;
        assert_eq!(mem::size_of::<ControlMsg>(), 16);
        assert_eq!(mem::align_of::<ControlMsg>(), 8);
    }
}
