use std::io::{Error, Result};
use std::mem::take;
use std::net::SocketAddr;

use crate::fs::File;

pub(crate) fn socket(addr: SocketAddr, ty: i32, protocol: i32) -> Result<i32> {
    use libc::*;

    let domain = match addr {
        SocketAddr::V4(_) => libc::AF_INET,
        SocketAddr::V6(_) => libc::AF_INET6,
    };

    let fd = unsafe { socket(domain, ty, protocol) };
    if fd < 0 {
        Err(Error::last_os_error())
    } else {
        Ok(fd)
    }
}

/// A type with the same memory layout as `libc::sockaddr`. Used in converting Rust level
/// SocketAddr* types into their system representation. The benefit of this specific
/// type over using `libc::sockaddr_storage` is that this type is exactly as large as it
/// needs to be and not a lot larger. And it can be initialized cleaner from Rust.
// Copied from mio.
#[repr(C)]
pub(crate) union SocketAddrCRepr {
    v4: libc::sockaddr_in,
    v6: libc::sockaddr_in6,
}

impl SocketAddrCRepr {
    pub(crate) fn as_ptr(&self) -> *const libc::sockaddr {
        self as *const _ as *const libc::sockaddr
    }
}

pub fn invalid_input() -> Error {
    Error::new(
        std::io::ErrorKind::InvalidInput,
        "could not resolve to any addresses",
    )
}

/// Converts a Rust `SocketAddr` into the system representation.
pub(crate) fn socket_addr(addr: &SocketAddr) -> (SocketAddrCRepr, libc::socklen_t) {
    match addr {
        SocketAddr::V4(ref addr) => {
            // `s_addr` is stored as BE on all machine and the array is in BE order.
            // So the native endian conversion method is used so that it's never swapped.
            let sin_addr = libc::in_addr {
                s_addr: u32::from_ne_bytes(addr.ip().octets()),
            };

            let sockaddr_in = libc::sockaddr_in {
                sin_family: libc::AF_INET as libc::sa_family_t,
                sin_port: addr.port().to_be(),
                sin_addr,
                sin_zero: [0; 8],
                #[cfg(any(
                    target_os = "dragonfly",
                    target_os = "freebsd",
                    target_os = "ios",
                    target_os = "macos",
                    target_os = "netbsd",
                    target_os = "openbsd"
                ))]
                sin_len: 0,
            };

            let sockaddr = SocketAddrCRepr { v4: sockaddr_in };
            let socklen = std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
            (sockaddr, socklen)
        }
        SocketAddr::V6(ref addr) => {
            let sockaddr_in6 = libc::sockaddr_in6 {
                sin6_family: libc::AF_INET6 as libc::sa_family_t,
                sin6_port: addr.port().to_be(),
                sin6_addr: libc::in6_addr {
                    s6_addr: addr.ip().octets(),
                },
                sin6_flowinfo: addr.flowinfo(),
                sin6_scope_id: addr.scope_id(),
                #[cfg(any(
                    target_os = "dragonfly",
                    target_os = "freebsd",
                    target_os = "ios",
                    target_os = "macos",
                    target_os = "netbsd",
                    target_os = "openbsd"
                ))]
                sin6_len: 0,
                #[cfg(target_os = "illumos")]
                __sin6_src_id: 0,
            };

            let sockaddr = SocketAddrCRepr { v6: sockaddr_in6 };
            let socklen = std::mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t;
            (sockaddr, socklen)
        }
    }
}

pub fn remove_comment(line: &[u8]) -> &[u8] {
    let Some(i) = line
        .iter()
        .enumerate()
        .find(|(_, c)| **c == b'#')
        .map(|(i, _)| i) else {
            return line;
        };
    &line[..i]
}

pub fn is_whitespace(c: &u8) -> bool {
    matches!(c, b'\t' | b'\n' | b'\x0C' | b'\r' | b' ')
}

pub async fn lines(path: &str, capacity: usize) -> Result<LineReader> {
    Ok(LineReader {
        file: File::open(path).await?,
        buf: Vec::with_capacity(capacity),
        seek: 0,
        char: 0,
    })
}

pub struct LineReader {
    file: File,
    buf: Vec<u8>,
    seek: usize,
    char: usize,
}

impl LineReader {
    fn try_read_line(&mut self) -> Option<*const [u8]> {
        let buf = &self.buf[self.char..];
        let (loc, _) = buf.iter().enumerate().find(|(_, c)| **c == b'\n')?;
        self.char += loc;
        Some(&buf[..self.char + 1])
    }

    pub async fn next(&mut self) -> Result<Option<&[u8]>> {
        let line = self.try_read_line();
        if line.is_some() {
            return Ok(line.map(|ptr| unsafe { &*ptr }));
        }
        self.fetch().await?;
        Ok(self.try_read_line().map(|ptr| unsafe { &*ptr }))
    }

    async fn fetch(&mut self) -> Result<()> {
        self.seek += self.char;
        self.char = 0;
        let (res, buf) = self.file.read_at(take(&mut self.buf), self.seek as _).await;
        self.buf = buf;
        res?;
        Ok(())
    }
}
