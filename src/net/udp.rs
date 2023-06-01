use crate::buf::{IoBuf, IoBufMut};
use crate::shared_driver::submit;

use super::utils::{invalid_input, socket_addr};

use io_uring::opcode::{Connect, Read, Recv, SendMsg, Write};
use io_uring::types::Fd;

use libc::{iovec, msghdr};
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::future::Future;
use std::io::Result;
use std::mem::zeroed;
use std::net::{SocketAddr, ToSocketAddrs};
use std::os::fd::AsRawFd;

pub struct UdpSocket {
    socket: socket2::Socket,
}

impl UdpSocket {
    // TODO make dns resolution async
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> Result<UdpSocket> {
        try_until_success(addr, |addr| async move {
            let domain = Domain::for_address(addr);
            UdpSocket::_bind(domain, addr)
        })
        .await
    }

    pub fn _bind(domain: Domain, addr: SocketAddr) -> Result<UdpSocket> {
        let ty = Type::DGRAM;
        let protocol = Protocol::UDP;
        let socket = Socket::new(domain, ty, Some(protocol))?;
        socket.bind(&addr.into())?;
        Ok(UdpSocket { socket })
    }

    pub async fn connect<A: ToSocketAddrs>(&self, addr: A) -> Result<()> {
        let fd = self.socket.as_raw_fd();
        try_until_success(addr, |addr| async move {
            let addr: SockAddr = addr.into();
            #[cfg(not(feature = "unsafe_completion"))]
            let addr = Box::new(addr);
            let sqe = Connect::new(Fd(fd), addr.as_ptr(), addr.len()).build();
            let (res, _) = unsafe { submit(sqe, addr).await };
            res?;
            Ok(())
        })
        .await
    }
    /// The recv() call is normally used only on a connected socket (see connect(2)). It is equivalent to the call:
    pub async fn recv<B: IoBufMut>(&self, mut buf: B) -> (Result<usize>, B) {
        let fd = self.socket.as_raw_fd();
        let len = buf.bytes_total() as u32;
        let ptr = buf.stable_mut_ptr();
        let sqe = Recv::new(Fd(fd), ptr, len).build();
        let (res, buf) = unsafe { submit(sqe, buf).await };
        let res = res.map(|r| r.result() as usize);
        (res, buf)
    }

    pub async fn read<B: IoBufMut>(&self, mut buf: B) -> (Result<usize>, B) {
        let fd = self.socket.as_raw_fd();
        let len = buf.bytes_total() as u32;
        let ptr = buf.stable_mut_ptr();
        let sqe = Read::new(Fd(fd), ptr, len).build();
        let (res, buf) = unsafe { submit(sqe, buf).await };
        let res = res.map(|r| r.result() as usize);
        (res, buf)
    }

    pub async fn write<B: IoBuf>(&self, buf: B) -> (Result<usize>, B) {
        let fd = self.socket.as_raw_fd();
        let len = buf.bytes_init() as u32;
        let ptr = buf.stable_ptr();
        let sqe = Write::new(Fd(fd), ptr, len).build();
        let (res, buf) = unsafe { submit(sqe, buf).await };
        let res = res.map(|r| r.result() as usize);
        (res, buf)
    }

    pub async fn send_to<B: IoBuf>(&self, buf: B, addr: SocketAddr) -> (Result<usize>, B) {
        let fd = self.socket.as_raw_fd();
        // we define the iovec from the buffer
        let msg_iov: iovec = iovec {
            iov_base: buf.stable_ptr().cast_mut().cast(),
            iov_len: buf.bytes_init(),
        };

        let msghdr: msghdr = unsafe { zeroed() };

        let (addr, len) = socket_addr(&addr);

        // we allocate everything once
        let mut msg = Box::new((msghdr, msg_iov, addr));

        // we set the address to point to the box
        msg.0.msg_name = &mut msg.2 as *mut _ as *mut _;
        msg.0.msg_namelen = len;

        // we set the iovec to point to the box
        msg.0.msg_iov = &mut msg.1;
        msg.0.msg_iovlen = 1;

        let sqe = SendMsg::new(Fd(fd), &msg.0).build();
        let (res, (_, buf)) = unsafe { submit(sqe, (msg, buf)).await };
        let res = res.map(|sqe| sqe.result() as usize);
        (res, buf)
    }
}

async fn try_until_success<A, T, F, Ft>(addr: A, mut f: F) -> Result<T>
where
    A: ToSocketAddrs,
    F: FnMut(SocketAddr) -> Ft,
    Ft: Future<Output = Result<T>>,
{
    let mut error = None;
    for addr in addr.to_socket_addrs()? {
        let result = f(addr).await;
        let Err(err) = result else {
                return result;
            };
        error = Some(err);
    }
    Err(error.unwrap_or_else(invalid_input))
}

#[test]
fn udp_server_and_client() {
    crate::block_on(async {
        let first_addr: SocketAddr = "127.0.0.1:2401".parse().unwrap();
        let second_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();

        // bind sockets
        let alice = UdpSocket::bind(first_addr).await?;
        let bob = UdpSocket::bind(second_addr).await?;

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
