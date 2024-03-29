#![allow(clippy::upper_case_acronyms)]
use std::io::Result;
use std::mem::{forget, size_of_val};
use std::net::{Shutdown, SocketAddr};
use std::os::fd::{FromRawFd, IntoRawFd};

use crate::buf::{IoBuf, IoBufMut};
use crate::detach;
use crate::reactor::op::{self};
use crate::utils::futures::not_thread_safe;
use crate::utils::syscall;

use libc::{SOL_SOCKET, SO_REUSEPORT};

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
    SEQPACKET = libc::SOCK_SEQPACKET,
}

#[repr(i32)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]

pub enum Protocol {
    #[default]
    IP = libc::IPPROTO_IP,
    TCP = libc::IPPROTO_TCP,
    UDP = libc::IPPROTO_UDP,
    ICMP = libc::IPPROTO_ICMP,
    ICMPV6 = libc::IPPROTO_ICMPV6,
}

pub struct Socket {
    pub fd: i32,
}

impl Socket {
    /// Creates a new socket
    pub async fn new(domain: Domain, ty: Type, proto: Protocol) -> Result<Self> {
        let fd = op::socket(domain as i32, ty as i32, proto as _, None)?;
        Ok(Self {
            fd: fd.into_raw_fd(),
        })
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

    pub async fn shutdown(&self, how: Shutdown) -> Result<()> {
        op::shutdown(self.fd, how).await?;
        Ok(())
    }

    pub fn bind(&self, addr: &SocketAddr) -> Result<()> {
        let (addr, len) = socket_addr(addr);
        syscall!(bind, self.fd, &addr as *const _ as _, len)?;
        Ok(())
    }

    pub fn listen(&self, backlog: u32) -> Result<()> {
        syscall!(listen, self.fd, backlog as i32)?;
        Ok(())
    }

    pub fn set_reuseport(&self) -> Result<()> {
        let optval = &1;
        let size = size_of_val(optval) as u32;
        let fd = self.fd;
        dbg!(self.fd);
        syscall!(
            setsockopt,
            fd,
            SOL_SOCKET,
            SO_REUSEPORT,
            optval as *const _ as *const _,
            size
        )?;
        Ok(())
    }

    pub async fn accept(&self) -> Result<(Socket, SocketAddr)> {
        let (fd, addr) = op::accept(self.fd).await?;
        let fd = fd.into_raw_fd();
        Ok((Socket { fd }, addr))
    }

    pub async fn close(self) -> Result<()> {
        not_thread_safe().await;
        let fd = self.fd;
        forget(self);
        op::close(fd).await
    }
}

impl FromRawFd for Socket {
    unsafe fn from_raw_fd(fd: std::os::fd::RawFd) -> Self {
        Self { fd }
    }
}
impl IntoRawFd for Socket {
    fn into_raw_fd(self) -> std::os::fd::RawFd {
        let fd = self.fd;
        forget(self);
        fd
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
