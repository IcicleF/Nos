//! Control-plane utilities.
//!
//! This module contains a cluster manager, a distributed barrier, and a thread
//! number manager.

mod cluster;
mod dist_barrier;
mod nthreads;

pub use cluster::*;
pub use dist_barrier::DistBarrier;
pub use nthreads::NThreads;

/// RDMA NIC configuration.
///
/// Prefer using the `tweak-nic.sh` script to change the NIC configuration,
/// instead of directly changing the constants in this file.
mod nic_config {
    /// The RDMA NIC to use.
    pub const NIC_NAME: &str = "mlx5_2";

    /// The physical port of the RDMA NIC to use.
    pub const NIC_PHYPORT: u8 = 1;
}

pub use nic_config::*;
