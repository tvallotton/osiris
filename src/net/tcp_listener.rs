use std::{io::Result, net::SocketAddr};

use crate::net::socket::{Domain, Protocol, Type};

use super::{socket::Socket, TcpStream};

pub struct TcpListener {
    socket: Socket,
}

impl TcpListener {
    pub fn bind(addr: SocketAddr) -> Result<TcpListener> {
        let domain = Domain::from(addr);
        let socket = Socket::new(domain, Type::STREAM, Protocol::TCP)?;
        socket.bind(&addr)?;
        Ok(TcpListener { socket })
    }

    pub async fn accept(&self) -> Result<(TcpStream, SocketAddr)> {
        let (socket, addr) = self.socket.accept().await?;
        Ok((TcpStream { socket }, addr))
    }
}
