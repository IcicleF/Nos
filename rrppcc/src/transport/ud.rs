use std::collections::VecDeque;
use std::mem;
use std::ptr::{self, NonNull};

use rrddmma::{
    bindings::*,
    prelude::*,
    rdma::{
        mr::Permission,
        qp::{QpEndpoint, QpPeer},
    },
};

use super::rc::ControlMsg;
use crate::msgbuf::MsgBuf;
use crate::pkthdr::PacketHeader;
use crate::util::{huge_alloc::*, likely::*};

/// Memory region handle type.
pub type LKey = rrddmma::rdma::type_alias::LKey;
pub type RKey = rrddmma::rdma::type_alias::RKey;

/// An item to transmit.
pub(crate) struct TxItem {
    /// Peer for this packet.
    pub peer: *const QpPeer,

    /// Packet header.
    pub pkthdr: *const MsgBuf,

    /// Message buffer.
    pub msgbuf: *const MsgBuf,
}

/// Received but unreturned message metadata.
struct RxItem {
    /// Receive unit index.
    idx: u16,

    /// Message length in bytes.
    len: u16,
}

/// RDMA UD transport.
/// Processes small messages and control packets of large messages.
///
/// While this type mainly serves UD transport, it also provides basic RDMA context
/// support for the [`RcTransport`](super::rc::RcTransport).
pub(crate) struct UdTransport {
    /// The UD queue pair.
    qp: Qp,
    /// Local port information.
    port: Port,
    /// Memory region registry.
    mrs: Vec<Mr>,

    /// Send packet sequence number.
    /// Used to check whether to signal a batch of send requests.
    tx_pkt_idx: usize,
    /// Send SGE buffer.
    tx_sgl: Vec<[ibv_sge; 2]>,
    /// Send work request buffer.
    tx_wr: Vec<ibv_send_wr>,

    /// Recv memory buffer.
    /// Place after `mrs` to ensure that it is dropped after memory regions.
    #[allow(unused)]
    rx_buf: HugeAlloc,
    /// Recv SGE buffer.
    #[allow(unused)]
    rx_sgl: Vec<ibv_sge>,
    /// Recv work request buffer.
    rx_wr: Vec<ibv_recv_wr>,
    /// Recv work completion buffer.
    rx_wc: Vec<Wc>,
    /// Received but unreturned messages.
    rx_items: VecDeque<RxItem>,
    /// Number of pending receive work requests to repost.
    rx_repost_pending: usize,
    /// Rx balance (for debug use).
    rx_balance: isize,
}

const CACHELINE_SIZE: usize = 64;

impl UdTransport {
    const GRH_SIZE: usize = 40;
    const MTU: usize = 1 << 12;
    const MAX_PKT_SIZE: usize = Self::MTU - mem::size_of::<PacketHeader>();

    const SQ_SIZE: usize = 1 << 8;
    const SQ_SIGNAL_BATCH: usize = 1 << 6;

    const RQ_SIZE: usize = 1 << 12;
    const RQ_POLL_BATCH: usize = 1 << 4;
    const RQ_POSTLIST_SIZE: usize = 1 << 4;
    const RX_UNIT_ALLOC_SIZE: usize = CACHELINE_SIZE + Self::MTU;
    const RX_UNIT_SIZE: usize = Self::GRH_SIZE + Self::MTU;
}

const AUXDATA_FLAG: u64 = 1 << 32;

impl UdTransport {
    /// Get the offset of the `i`-th receive unit in the entire buffer.
    /// This will point to the beginning of the GRH.
    #[inline(always)]
    const fn rx_offset(i: usize) -> usize {
        i * UdTransport::RX_UNIT_ALLOC_SIZE + (CACHELINE_SIZE - UdTransport::GRH_SIZE)
    }

    /// Get the offset of the `i`-th receive unit's payload in the entire buffer.
    /// This will point to the beginning of the payload, usually the packet header.
    #[inline(always)]
    const fn rx_payload_offset(i: usize) -> usize {
        i * UdTransport::RX_UNIT_ALLOC_SIZE + CACHELINE_SIZE
    }
}

impl UdTransport {
    /// Create a new transport instance that is bound to a specific port on a
    /// specific NIC.
    #[allow(clippy::assertions_on_constants)]
    pub fn new(nic: &str, phy_port: u8) -> Self {
        assert!(
            CACHELINE_SIZE >= Self::GRH_SIZE,
            "GRH too large, cannot fit in cacheline"
        );
        assert!(
            Self::RQ_SIZE.trailing_zeros() < LKey::BITS,
            "too many recv units, index cannot fit in LocalKey"
        );

        // Initialize QP.
        let Nic { context, ports } = Nic::finder()
            .dev_name(nic)
            .port_num(phy_port)
            .probe_nth_port(0)
            .expect("failed to find target NIC or physical port");
        let port = ports.into_iter().next().unwrap();
        assert!(
            port.mtu().bytes() == Self::mtu(),
            "path active MTU must be 4KiB"
        );

        let pd = Pd::new(&context).expect("failed to allocate protection domain");
        let qp = {
            let send_cq =
                Cq::new(&context, Self::SQ_SIZE as _).expect("failed to allocate UD send CQ");
            let recv_cq =
                Cq::new(&context, Self::RQ_SIZE as _).expect("failed to allocate UD recv CQ");
            let mut qp = Qp::builder()
                .qp_type(QpType::Ud)
                .send_cq(&send_cq)
                .recv_cq(&recv_cq)
                .caps(QpCaps {
                    max_send_wr: Self::SQ_SIZE as _,
                    max_recv_wr: Self::RQ_SIZE as _,
                    max_send_sge: 2,
                    max_recv_sge: 1,
                    ..Default::default()
                })
                .sq_sig_all(false)
                .build(&pd)
                .expect("failed to create UD queue pair");
            qp.bind_local_port(&port, None)
                .expect("failed to bind UD QP to port");
            qp
        };

        // Initialize send WRs.
        let mut tx_sgl = vec![[ibv_sge::default(); 2]; Self::SQ_SIGNAL_BATCH + 1];
        let mut tx_wr = (0..(Self::SQ_SIGNAL_BATCH + 1))
            .map(|i| ibv_send_wr {
                wr_id: i as _,
                sg_list: tx_sgl[i].as_mut_ptr(),
                num_sge: 2,
                opcode: ibv_wr_opcode::IBV_WR_SEND,
                ..unsafe { mem::zeroed() }
            })
            .collect::<Vec<_>>();
        for i in 0..Self::SQ_SIGNAL_BATCH {
            tx_wr[i].next = &mut tx_wr[i + 1] as *mut _;
        }

        // Initialize recv buffer.
        let rx_buf = alloc_raw(Self::RQ_SIZE * Self::RX_UNIT_ALLOC_SIZE);
        assert!(rx_buf.ptr as usize % CACHELINE_SIZE == 0);

        // SAFETY: correct allocated buffer.
        let rx_mr = unsafe {
            Mr::reg(&pd, rx_buf.ptr, rx_buf.len, Permission::LOCAL_WRITE)
                .expect("failed to register recv memory region")
        };

        // Initialize recv WRs.
        let mut rx_sge = (0..Self::RQ_POSTLIST_SIZE)
            .map(|_| ibv_sge {
                addr: 0,
                length: Self::RX_UNIT_SIZE as _,
                lkey: rx_mr.lkey(),
            })
            .collect::<Vec<_>>();

        let mut rx_wr = (0..Self::RQ_POSTLIST_SIZE)
            .map(|i| ibv_recv_wr {
                sg_list: &mut rx_sge[i],
                num_sge: 1,
                ..unsafe { mem::zeroed() }
            })
            .collect::<Vec<_>>();
        for i in 0..(Self::RQ_POSTLIST_SIZE - 1) {
            rx_wr[i].next = &mut rx_wr[i + 1];
        }

        // Post recv WRs in batches.
        for start in (0..Self::RQ_SIZE).step_by(Self::RQ_POSTLIST_SIZE) {
            let end = (start + Self::RQ_POSTLIST_SIZE).min(Self::RQ_SIZE);

            for i in 0..(end - start) {
                let offset = Self::rx_offset(start + i);

                // SAFETY: in the same allocated buffer.
                rx_sge[i].addr = unsafe { rx_buf.ptr.add(offset) } as _;
                rx_wr[i].wr_id = (start + i) as _;
            }

            // SAFETY: FFI.
            unsafe {
                qp.post_raw_recv(&rx_wr[0])
                    .expect("failed to post recv WRs");
            }
        }

        // Initialize recv WC buffer.
        let rx_wc = vec![Wc::default(); Self::RQ_POLL_BATCH];
        let rx_items = VecDeque::with_capacity(Self::RQ_POLL_BATCH);

        Self {
            qp,
            port,
            mrs: vec![rx_mr],

            tx_pkt_idx: 0,
            tx_sgl,
            tx_wr,

            rx_buf,
            rx_sgl: rx_sge,
            rx_wr,
            rx_wc,
            rx_items,
            rx_repost_pending: 0,
            rx_balance: 0,
        }
    }

    /// Return the MTU of the transport.
    /// The MTU is the maximum data amount of a single packet.
    #[inline(always)]
    pub const fn mtu() -> usize {
        Self::MTU
    }

    /// Return the maximum amount of data that can be sent in a single packet.
    /// This is the MTU minus the size of the packet header.
    #[inline(always)]
    pub const fn max_data_in_pkt() -> usize {
        Self::MAX_PKT_SIZE
    }

    /// Return the RDMA context used by this transport instance.
    pub fn pd(&self) -> &Pd {
        self.qp.pd()
    }

    /// Return serialized endpoint information representing the transport instance.
    #[cold]
    pub fn endpoint(&self) -> QpEndpoint {
        self.qp.endpoint().unwrap()
    }

    /// Return the port information that the UD QP is bound to.
    pub fn port(&self) -> &Port {
        &self.port
    }

    /// Construct a peer from the given endpoint information.
    #[cold]
    pub fn create_peer(&self, ep: QpEndpoint) -> QpPeer {
        self.qp
            .make_peer(&ep)
            .unwrap_or_else(|_| panic!("failed to create peer from endpoint {:?}", ep))
    }

    /// Register memory so that it is accessible by the transport.
    /// Return a handle to the registered memory region.
    ///
    /// # Safety
    ///
    /// The memory region `[buf, buf + len)` must be valid for access.
    #[cold]
    pub unsafe fn reg_mem(&mut self, buf: *mut u8, len: usize) -> (LKey, RKey) {
        let mr = Mr::reg(self.qp.pd(), buf, len, Permission::default())
            .expect("failed to register memory region");
        let keys = (mr.lkey(), mr.rkey());
        self.mrs.push(mr);
        keys
    }

    /// Transmit a batch of messages.
    /// Drain the send DMA queue if `drain` is set to true.
    ///
    /// # Safety
    ///
    /// The items in the batch must all be valid.
    pub unsafe fn tx_burst(&mut self, items: &[TxItem], drain: bool) {
        if unlikely(items.is_empty()) {
            return;
        }

        // Split the many packets into batches.
        // Batch size must not be larger than `SQ_SIGNAL_BATCH`, or it will stuck
        // when trying to poll CQEs from unposted WRs and break the invariant that
        // there is always exactly one unpolled CQE (after the first Tx packet).
        if unlikely(items.len() > Self::SQ_SIGNAL_BATCH) {
            let chunks = items.chunks(Self::SQ_SIGNAL_BATCH);
            let n = chunks.len();

            // SAFETY: recursion (induction).
            for (i, chunk) in chunks.enumerate() {
                self.tx_burst(chunk, drain && (i + 1 == n));
            }
            return;
        }

        // Now we are sure that the batch size is within the `SQ_SIGNAL_BATCH` limit.
        for (i, item) in items.iter().enumerate() {
            // SAFETY: the caller ensures that memory handles are valid.
            let sgl = &mut self.tx_sgl[i];
            let wr = &mut self.tx_wr[i];
            debug_assert_eq!(wr.sg_list, sgl as *mut _);
            debug_assert_eq!(wr.num_sge, 2);

            // Set signaled flag + poll send CQ if needed.
            wr.send_flags = if self.tx_pkt_idx % Self::SQ_SIGNAL_BATCH == 0 {
                if likely(self.tx_pkt_idx > 0) {
                    self.qp.scq().poll_one_blocking_consumed();
                }
                ibv_send_flags::IBV_SEND_SIGNALED.0
            } else {
                0
            };
            self.tx_pkt_idx += 1;

            // Fill in the scatter/gather list.
            let mut length = mem::size_of::<PacketHeader>() as u32;

            // 1. Packet header.
            let pkthdr = &*item.pkthdr;
            sgl[0] = ibv_sge {
                addr: pkthdr.as_ptr() as _,
                length: mem::size_of::<PacketHeader>() as _,
                lkey: pkthdr.lkey(),
            };

            // 2. Payload: data if small, control otherwise.
            let msgbuf = &*item.msgbuf;
            if likely(msgbuf.is_small()) {
                length += msgbuf.len() as u32;
                sgl[1] = ibv_sge {
                    addr: msgbuf.as_ptr() as _,
                    length: msgbuf.len() as _,
                    lkey: msgbuf.lkey(),
                };
            } else {
                length += mem::size_of::<ControlMsg>() as u32;
                sgl[1] = ibv_sge {
                    addr: msgbuf.ctrl_msg() as _,
                    length: mem::size_of::<ControlMsg>() as _,
                    lkey: msgbuf.lkey(),
                };
            }

            // Inline check.
            if length <= self.qp.caps().max_inline_data {
                wr.send_flags |= ibv_send_flags::IBV_SEND_INLINE.0;
            }

            // Fill in routing information.
            // Safety requirements should be upheld by the caller, no need to check here.
            wr.wr.ud = unsafe { (*item.peer).ud() };
        }

        // Break the linked list chain.
        self.tx_wr[items.len() - 1].next = ptr::null_mut();

        // If we need to drain the send DMA queue, the last WR must be signaled.
        // We also need to record the total number of outstanding signaled WRs.
        let need_poll = if unlikely(drain) {
            let last_wr = &mut self.tx_wr[items.len() - 1];
            if last_wr.send_flags & ibv_send_flags::IBV_SEND_SIGNALED.0 != 0 {
                1
            } else {
                last_wr.send_flags |= ibv_send_flags::IBV_SEND_SIGNALED.0;
                2
            }
        } else {
            0
        };

        // SAFETY: all work requests are correctly constructed.
        self.qp
            .post_raw_send(&self.tx_wr[0])
            .expect("failed to post send WRs");

        // Restore the linked list chain.
        self.tx_wr[items.len() - 1].next = &mut self.tx_wr[items.len()] as *mut _;

        // Poll send CQ and check the completions if we need to drain the send DMA queue.
        if unlikely(drain) {
            for _ in 0..need_poll {
                self.qp.scq().poll_one_blocking_consumed();
            }
            self.tx_pkt_idx = 0;
        }
    }

    /// Receive a batch of messages.
    /// Return the number of messages received.
    pub fn rx_burst(&mut self) -> usize {
        let n = self
            .qp
            .rcq()
            .poll_into(&mut self.rx_wc)
            .expect("failed to poll recv CQ") as usize;
        for i in 0..n {
            let wc = &self.rx_wc[i];
            let byte_len = wc.ok().expect("failed to recv");
            self.rx_items.push_back(RxItem {
                idx: wc.wr_id() as _,
                len: (byte_len - Self::GRH_SIZE - mem::size_of::<PacketHeader>()) as _,
            });
        }
        n
    }

    /// Return the next received message.
    #[inline]
    pub fn rx_next(&mut self) -> Option<MsgBuf> {
        let RxItem { idx, len } = self.rx_items.pop_front()?;
        let offset = Self::rx_payload_offset(idx as _);
        self.rx_balance += 1;

        // SAFETY: pointer guaranteed not-null, and within the same allocated buffer.
        let data = unsafe {
            NonNull::new_unchecked(
                self.rx_buf.ptr.add(offset + mem::size_of::<PacketHeader>()) as *mut _
            )
        };

        // Embed the index into the auxiliary data field.
        let mut msgbuf = MsgBuf::borrowed(data, len as _);
        msgbuf.aux_data = (idx as u64) | AUXDATA_FLAG;
        Some(msgbuf)
    }

    /// Mark a received message as released and can be reused.
    ///
    /// # Safety
    ///
    /// - Only `MsgBuf` returned by `rx_next` can be released.
    /// - Every `MsgBuf` must not be used after it is released.
    /// - Every `MsgBuf` must not be released more than once.
    #[inline]
    pub unsafe fn rx_release(&mut self, item: &MsgBuf) {
        self.rx_balance -= 1;
        assert!(
            self.rx_balance >= 0,
            "rx balance underflowed by: {:#?}",
            item
        );

        let i = self.rx_repost_pending;

        // SAFETY: in the same allocated buffer.
        let idx = item.aux_data;
        assert_ne!(
            idx & AUXDATA_FLAG,
            0,
            "invalid rx buffer to release: {:#?}",
            item
        );
        let idx = idx as u16 as u64;

        self.rx_sgl[i].addr = unsafe { self.rx_buf.ptr.add(Self::rx_offset(idx as usize)) } as _;
        self.rx_wr[i].wr_id = idx;
        self.rx_repost_pending += 1;

        if unlikely(self.rx_repost_pending == Self::RQ_POSTLIST_SIZE) {
            // SAFETY: all work requests are correctly constructed.
            self.qp
                .post_raw_recv(&self.rx_wr[0])
                .expect("failed to post recv WRs");
            self.rx_repost_pending = 0;
        }
    }
}
