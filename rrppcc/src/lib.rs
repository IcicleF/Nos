//! # rrppcc
//!
#![doc = include_str!("../README.md")]
//!
//! ## Example
//!
//! This example sets up a server and a client on two threads, and sends a request
//! from the client to the server. It assumes that the UDP port 31850 and 31851 can
//! be used.
//!
//! ```rust,no_run
#![doc = include_str!("../examples/hello.rs")]
//! ```

mod handler;
mod msgbuf;
mod nexus;
mod pkthdr;
mod request;
mod rpc;
mod session;
mod transport;
pub mod type_alias;
mod util;

pub use self::msgbuf::MsgBuf;
pub use self::nexus::Nexus;
pub use self::request::Request;
pub use self::rpc::Rpc;
pub use self::session::SessionHandle as Session;

pub use self::handler::RequestHandle;

#[cfg(test)]
mod tests;
