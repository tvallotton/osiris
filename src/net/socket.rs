#![allow(clippy::upper_case_acronyms)]
use std::{
    io::{Error, Result},
    mem::forget,
};

use crate::{detach, reactor::op};
use libc::{socket, SOCK_CLOEXEC};

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
#[derive(Default)]

pub enum Protocol {
    #[default]
    UNSPECIFIED = 0,
    TCP = libc::IPPROTO_TCP,
    UDP = libc::IPPROTO_UDP,
    MPTCP = libc::IPPROTO_MPTCP,
    ICMP = libc::IPPROTO_ICMP,
    ICMPV6 = libc::IPPROTO_ICMPV6,
}

pub struct Socket {
    fd: i32,
}

impl Socket {
    /// Creates a new socket
    pub fn new(domain: Domain, ty: Type, proto: Protocol) -> Result<Self> {
        let fd = unsafe { socket(domain as i32, ty as i32 | SOCK_CLOEXEC, proto as i32) };
        if fd == -1 {
            return Err(Error::last_os_error());
        }
        Ok(Self { fd })
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
