//! RPC types.

use rrppcc::type_alias::ReqType;

// ==============================================
/// Start of normal data-plane RPC types.
pub const BEGIN_DATA_RPC: ReqType = 10;

/// Data write.
pub const RPC_DATA_WRITE: ReqType = 11;

/// Data read.
/// Will become a degraded read if sent to a non-primary.
pub const RPC_DATA_READ: ReqType = 12;

/// Metadata read.
pub const RPC_DATA_SPLIT_READ: ReqType = 13;

/// End of normal data-plane RPC types.
pub const END_DATA_RPC: ReqType = 99;

// ==============================================
/// Start of recovery RPC types.
pub const BEGIN_RECOVERY_RPC: ReqType = 100;

/// Encodee list fetch.
pub const RPC_ENCODEE_LIST_FETCH: ReqType = 100;

/// Parity read.
pub const RPC_PARITY_READ: ReqType = 101;

/// End of recovery RPC types.
pub const END_RECOVERY_RPC: ReqType = 199;

// ==============================================
/// Start of control RPC types.
pub const BEGIN_CONTROL_RPC: ReqType = 200;

/// Make a peer logically failed.
pub const RPC_MAKE_FAILED: ReqType = 201;

/// Make a peer logically alive.
pub const RPC_MAKE_ALIVE: ReqType = 202;

/// Make a peer logically slowed down.
pub const RPC_MAKE_SLOWED: ReqType = 203;

/// Make a peer logically recovered from slow-down.
pub const RPC_MAKE_SLOW_RECOVERED: ReqType = 204;

/// Let a peer report its latency statistics.
pub const RPC_REPORT_STATS: ReqType = 210;

/// Fake MDS access.
pub const RPC_FAKE_MDS_REQUEST: ReqType = 211;

/// Asks the receiver to halt.
pub const RPC_HALT: ReqType = 250;

/// End of control RPC types.
pub const END_CONTROL_RPC: ReqType = ReqType::MAX - 1;

/// A list of all possible RPC types.
pub static ALL_RPC_TYPES: &[ReqType] = &[
    // Data RPC types.
    RPC_DATA_WRITE,
    RPC_DATA_READ,
    RPC_DATA_SPLIT_READ,
    // Recovery RPC types.
    RPC_ENCODEE_LIST_FETCH,
    RPC_PARITY_READ,
    // Control RPC types.
    RPC_MAKE_FAILED,
    RPC_MAKE_ALIVE,
    RPC_MAKE_SLOWED,
    RPC_MAKE_SLOW_RECOVERED,
    RPC_REPORT_STATS,
    RPC_FAKE_MDS_REQUEST,
    RPC_HALT,
];
