#[cfg(io_uring)]
use crate::fs::File;
use libc::{AF_INET, AF_INET6};
use memchr::memchr;
use std::io::{Error, Result};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::ptr::addr_of;

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

/// Returns this address as a `SocketAddr` if it is in the `AF_INET` (IPv4)
/// or `AF_INET6` (IPv6) family, otherwise returns `None`.
pub fn to_std_socket_addr(storage: &libc::sockaddr) -> Result<SocketAddr> {
    if storage.sa_family == AF_INET as _ {
        // SAFETY: if the `ss_family` field is `AF_INET` then storage must
        // be a `sockaddr_in`.
        let addr: &libc::sockaddr_in = unsafe { &*(addr_of!(storage).cast()) };
        let port = u16::from_be(addr.sin_port);
        let ip = Ipv4Addr::from(addr.sin_addr.s_addr.to_ne_bytes());
        Ok(SocketAddr::V4(SocketAddrV4::new(ip, port)))
    } else if storage.sa_family == AF_INET6 as _ {
        // SAFETY: if the `ss_family` field is `AF_INET6` then storage must
        // be a `sockaddr_in6`.
        let addr: &libc::sockaddr_in6 = unsafe { &*(addr_of!(storage).cast()) };
        let ip = Ipv6Addr::from(addr.sin6_addr.s6_addr);
        let port = u16::from_be(addr.sin6_port);
        Ok(SocketAddr::V6(SocketAddrV6::new(
            ip,
            port,
            addr.sin6_flowinfo,
            #[cfg(unix)]
            addr.sin6_scope_id,
        )))
    } else {
        Err(Error::new(
            std::io::ErrorKind::Other,
            "unsupported IP version",
        ))
    }
}

pub fn remove_comment(line: &[u8]) -> &[u8] {
    let Some(i) = memchr(b'#', line) else {
        return line
    };
    &line[..i]
}

pub fn is_whitespace(c: &u8) -> bool {
    matches!(c, b'\t' | b'\n' | b'\x0C' | b'\r' | b' ')
}
#[cfg(io_uring)]
pub async fn lines(path: &str, capacity: usize) -> Result<LineReader> {
    let file = File::open(path).await?;

    Ok(LineReader {
        end: file.metadata().await?.len(),
        file,
        buf: Vec::with_capacity(capacity),
        seek: 0,
        char: 0,
    })
}
// TODO: refactor this to use file position?
#[cfg(io_uring)]
pub struct LineReader {
    file: File,
    buf: Vec<u8>,
    seek: usize,
    char: usize,
    end: usize,
}
#[cfg(io_uring)]
impl LineReader {
    fn try_read_line(&mut self) -> Option<*const [u8]> {
        let buf = self.buf.get(self.char..)?;
        let i = memchr(b'\n', buf)?;
        self.char += i + 1;
        Some(buf.get(..(i + 1))?)
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
        let (res, buf) = self.file.read_at(std::mem::take(&mut self.buf), self.seek as _).await;
        self.buf = buf;
        unsafe { self.buf.set_len(res?) };
        Ok(())
    }
}
