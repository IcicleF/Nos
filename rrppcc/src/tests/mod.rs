#![allow(unused_imports)]

use super::{type_alias::*, *};
use std::{
    array, ptr,
    sync::{atomic::*, *},
    thread,
};

use futures::executor::block_on;
use futures::future::join_all;
use rand::Rng;
use simple_logger::SimpleLogger;

static PORT: AtomicU16 = AtomicU16::new(31850);

#[inline]
pub(self) fn next_port() -> u16 {
    PORT.fetch_add(1, Ordering::SeqCst)
}

const NIC_NAME: &str = "mlx5_2";

mod control_plane;
mod corners;
mod large;
mod small;
