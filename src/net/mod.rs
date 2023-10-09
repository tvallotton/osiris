//! TCP/UDP bindings for `osiris`.
//!
//! This module contains the TCP/UDP/Unix networking types, similar to the standard
//! library, which can be used to implement networking protocols.
//!
//! # Organization
//!
//! * [`TcpListener`] and [`TcpStream`] provide functionality for communication over TCP
//! * [`UdpSocket`] provides functionality for communication over UDP

#[cfg(io_uring)]
mod dns;
pub(crate) mod pipe;
mod socket;
mod tcp_listener;
mod tcp_stream;
mod to_socket_addr;

mod udp;
pub(crate) mod utils;

pub use std::net::{Shutdown, SocketAddr};
pub use tcp_listener::TcpListener;
pub use tcp_stream::TcpStream;
pub use to_socket_addr::ToSocketAddrs;
pub use udp::UdpSocket;
