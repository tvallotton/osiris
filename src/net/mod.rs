mod dns;
mod socket;
mod tcp_listener;
mod tcp_stream;
mod to_socket_addr;
mod udp;
pub(crate) mod utils;

pub use std::net::Shutdown;
pub use std::net::SocketAddr;
pub use tcp_listener::TcpListener;
pub use tcp_stream::TcpStream;
pub use to_socket_addr::ToSocketAddrs;
pub use udp::UdpSocket;
