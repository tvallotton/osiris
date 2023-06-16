#![allow(clippy::upper_case_acronyms)]
use std::{
    io::{Error, Result},
    mem::forget,
    net::SocketAddr,
};

use crate::{
    buf::{IoBuf, IoBufMut},
    detach,
    reactor::op::{self},
};

use libc::SOCK_CLOEXEC;

use super::utils::socket_addr;

#[repr(i32)]
pub enum Domain {
    V4 = libc::AF_INET,
    V6 = libc::AF_INET6,
}

#[repr(i32)]
pub enum Type {
    STREAM = libc::SOCK_STREAM,
    DGRAM = libc::SOCK_DGRAM,
    RMD = libc::SOCK_RDM,
    RAW = libc::SOCK_RAW,
    PACKED = libc::SOCK_PACKET,
    SEQPACKET = libc::SOCK_SEQPACKET,
}

#[repr(i32)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]

pub enum Protocol {
    #[default]
    IP = libc::IPPROTO_IP,
    TCP = libc::IPPROTO_TCP,
    UDP = libc::IPPROTO_UDP,
    MPTCP = libc::IPPROTO_MPTCP,
    ICMP = libc::IPPROTO_ICMP,
    ICMPV6 = libc::IPPROTO_ICMPV6,
}

pub struct Socket {
    pub fd: i32,
}

impl Socket {
    /// Creates a new socket
    pub async fn new(domain: Domain, ty: Type, proto: Protocol) -> Result<Self> {
        let fd = unsafe { libc::socket(domain as _, SOCK_CLOEXEC | ty as i32, proto as _) };
        // TODO: Figure out why this fails
        // let fd = op::socket(domain as _, ty as i32, proto as _, None).await?;
        Ok(Self { fd })
    }

    pub async fn read<B: IoBufMut>(&self, buf: B) -> (Result<usize>, B) {
        op::read_at(self.fd, buf, 0).await
    }

    pub async fn write<B: IoBuf>(&self, buf: B) -> (Result<usize>, B) {
        op::write_at(self.fd, buf, 0).await
    }

    pub async fn recv<B: IoBufMut>(&self, buf: B) -> (Result<usize>, B) {
        op::recv(self.fd, buf).await
    }

    pub async fn connect(&self, addr: SocketAddr) -> Result<()> {
        op::connect(self.fd, addr).await
    }

    pub async fn send_to<B: IoBuf>(&self, buf: B, addr: SocketAddr) -> (Result<usize>, B) {
        op::send_to(self.fd, buf, addr).await
    }

    pub fn bind(&self, addr: &SocketAddr) -> Result<()> {
        let (addr, len) = socket_addr(addr);
        let res = unsafe { libc::bind(self.fd, &addr as *const _ as _, len) };
        if res == -1 {
            return Err(Error::last_os_error());
        }
        Ok(())
    }

    pub async fn close(self) -> Result<()> {
        let fd = self.fd;
        forget(self);
        op::close(fd).await
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        detach(op::close(self.fd));
    }
}

impl From<SocketAddr> for Domain {
    #[inline]
    fn from(value: SocketAddr) -> Self {
        match value {
            SocketAddr::V4(_) => Domain::V4,
            SocketAddr::V6(_) => Domain::V6,
        }
    }
}