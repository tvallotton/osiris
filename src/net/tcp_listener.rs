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
        socket.set_reuseport()?;
        socket.bind(&addr)?;
        socket.listen(128)?;
        Ok(TcpListener { socket })
    }

    pub async fn accept(&self) -> Result<(TcpStream, SocketAddr)> {
        let (socket, addr) = self.socket.accept().await?;
        Ok((TcpStream { socket }, addr))
    }
}

#[test]
fn reuseport() {
    crate::block_on(async {
        let _listener = TcpListener::bind("127.0.0.1:8080".parse().unwrap()).unwrap();
        let _listener2 = TcpListener::bind("127.0.0.1:8080".parse().unwrap()).unwrap();
    })
    .unwrap();
}
