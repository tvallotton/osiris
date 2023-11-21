use super::socket::{Protocol, Socket, Type};
use super::to_socket_addr::{try_until_success, ToSocketAddrs};
use crate::buf::{IoBuf, IoBufMut};
use std::io::Result;
use std::net::SocketAddr;

pub struct UdpSocket {
    socket: Socket,
}

impl UdpSocket {
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> Result<UdpSocket>
where {
        try_until_success(addr, |addr| async move {
            let domain = addr.into();
            let socket = Socket::new(domain, Type::DGRAM, Protocol::UDP).await?;
            socket.bind(&addr)?;
            Ok(UdpSocket { socket })
        })
        .await
    }

    pub async fn connect<A>(&self, addr: A) -> Result<()>
    where
        A: ToSocketAddrs,
    {
        try_until_success(addr, |addr| self.socket.connect(addr)).await
    }
    /// The recv() call is normally used only on a connected socket (see connect(2)). It is equivalent to the call:
    pub async fn recv<B: IoBufMut>(&mut self, buf: B) -> (Result<usize>, B) {
        self.socket.recv(buf).await
    }

    pub async fn read<B: IoBufMut>(&mut self, buf: B) -> (Result<usize>, B) {
        self.socket.read(buf).await
    }

    pub async fn write<B: IoBuf>(&mut self, buf: B) -> (Result<usize>, B) {
        self.socket.write(buf).await
    }

    pub async fn send_to<B: IoBuf>(&mut self, buf: B, addr: SocketAddr) -> (Result<usize>, B) {
        self.socket.send_to(buf, addr).await
    }
}

#[test]
fn udp_server_and_client() {
    crate::block_on(async {
        let first_addr: SocketAddr = "127.0.0.1:2401".parse().unwrap();
        let second_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();

        // bind sockets
        let mut alice = UdpSocket::bind(first_addr).await?;
        let mut bob = UdpSocket::bind(second_addr).await?;

        // connect sockets
        alice.connect(second_addr).await.unwrap();
        bob.connect(first_addr).await.unwrap();
        let buf = vec![0; 32];

        // write data
        let (result, _) = alice.write(b"hello bob".as_slice()).await;
        result.unwrap();

        // read data
        let (result, buf) = bob.read(buf).await;
        let n_bytes = result.unwrap();

        assert_eq!(b"hello bob", &buf[..n_bytes]);

        // write data using send on connected socket
        let (result, _) = bob.write(b"hello world via send".as_slice()).await;
        result.unwrap();

        // read data
        let (result, buf) = alice.read(buf).await;
        let n_bytes = result.unwrap();

        assert_eq!(b"hello world via send", &buf[..n_bytes]);

        Result::Ok(())
    })
    .unwrap()
    .unwrap();
}
