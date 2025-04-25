use std::{fmt, hint, mem};

use bitvec::{field::BitField, prelude as bv};
use paste::paste;

use crate::type_alias::*;

macro_rules! impl_accessor {
    (
        Field = $field:ident,
        Self = $SelfT:ty, 
        ActualT = $ActualT:ty,
        lsb = $lsb:expr, 
        msb = $msb:expr, 
        setter = $setter:tt,
        getter = $getter:tt) => {

        #[inline(always)]
        pub fn $setter(&mut self, val: $SelfT) {
            self.bits[$lsb..=$msb].store_le::<$ActualT>(val as $ActualT);
        }

        #[inline(always)]
        pub fn $getter(&self) -> $SelfT {
            self.bits[$lsb..=$msb].load_le::<$ActualT>().into()
        }
    };

    (
        Field = $field:ident,
        Self = $SelfT:ty,
        ActualT = $ActualT:ty,
        lsb = $lsb:expr, 
        msb = $msb:expr
    ) => {
        paste! {
            impl_accessor!(
                Field = $field,
                Self = $SelfT,
                ActualT = $ActualT,
                lsb = $lsb,
                msb = $msb,
                setter = [< set_ $field >],
                getter = $field
            );
        }
    };

    (
        Field = $field:ident,
        Self = $SelfT:ty,
        lsb = $lsb:expr,
        msb = $msb:expr
    ) => {
        paste! {
            impl_accessor!(
                Field = $field,
                Self = $SelfT,
                ActualT = $SelfT,
                lsb = $lsb,
                msb = $msb
            );
        }
    };
}

/// Packet type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum PktType {
    /// Single-packet request.
    SmallReq = 0,

    /// Control packet for a large request.
    LargeReq = 1,

    /// Single-packet response.
    SmallResp = 2,

    /// Control packet for a large response.
    LargeResp = 3,
}

impl From<u8> for PktType {
    fn from(val: u8) -> Self {
        match val {
            0 => Self::SmallReq,
            1 => Self::LargeReq,
            2 => Self::SmallResp,
            3 => Self::LargeResp,

            // SAFETY: only used by `PacketHeader::pkt_type()`, which will only
            // pass 2-bit values to this function.
            _ => unsafe { hint::unreachable_unchecked() },
        }
    }
}

/// Packet header, 16-bytes.
///
/// # Layout
///
/// | lsb | msb |     Name     |
/// | --: | --: | ------------ |
/// |   0 |   7 | req_type     |
/// |   8 |  31 | len          |
/// |  32 |  63 | dst_sess_id  |
/// |  64 | 125 | req_idx      |
/// | 126 | 127 | pkt_type     |
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub(crate) struct PacketHeader {
    bits: bv::BitArr!(for 128),
}

impl PacketHeader {
    impl_accessor!(Field = req_type, Self = ReqType, lsb = 0, msb = 7);
    impl_accessor!(Field = data_len, Self = u32, lsb = 8, msb = 31);
    impl_accessor!(Field = dst_sess_id, Self = SessId, lsb = 32, msb = 63);
    impl_accessor!(Field = req_idx, Self = ReqIdx, lsb = 64, msb = 125);
    impl_accessor!(Field = pkt_type, Self = PktType, ActualT = u8, lsb = 126, msb = 127);

    pub fn new(
        req_type: ReqType,
        data_len: u32,
        dst_sess_id: SessId,
        req_idx: ReqIdx,
        pkt_type: PktType,
    ) -> Self {
        let mut this = Self::default();
        this.set_req_type(req_type);
        this.set_data_len(data_len);
        this.set_dst_sess_id(dst_sess_id);
        this.set_req_idx(req_idx);
        this.set_pkt_type(pkt_type);
        this
    }
}

impl fmt::Debug for PacketHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PacketHeader")
            .field("dst_sess_id", &self.dst_sess_id())
            .field("pkt_type", &self.pkt_type())
            .field("req_type", &self.req_type())
            .field("req_idx", &self.req_idx())
            .field("len", &self.data_len())
            .finish()
    }
}

impl Default for PacketHeader {
    fn default() -> Self {
        Self {
            bits: bv::bitarr![0u8; mem::size_of::<PacketHeader>() * 8],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_layout() {
        assert_eq!(mem::size_of::<PacketHeader>(), 16); 
        assert_eq!(mem::align_of::<PacketHeader>(), 8);
    }
}
