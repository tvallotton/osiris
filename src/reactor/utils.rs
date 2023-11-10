use std::convert::Infallible;
use std::io::Result;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

use crate::utils::syscall;

pub fn socket(domain: i32, ty: i32, proto: i32, _: Option<Infallible>) -> Result<OwnedFd> {
    let fd = syscall!(socket, domain as _, ty, proto as _)?;
    let fd = unsafe { OwnedFd::from_raw_fd(fd) };
    make_nonblocking(&fd)?;
    Ok(fd)
}

pub fn make_nonblocking(fd: &OwnedFd) -> Result<()> {
    let fd = fd.as_raw_fd();
    let flags = syscall!(fcntl, fd, libc::F_GETFL)?;
    syscall!(fcntl, fd, libc::F_SETFL, flags | libc::O_NONBLOCK)?;
    Ok(())
}

pub fn make_blocking(fd: &OwnedFd) -> Result<()> {
    let fd = fd.as_raw_fd();
    let options = syscall!(fcntl, fd, libc::F_GETFL)?;
    syscall!(fcntl, fd, libc::F_SETFL, options & !libc::O_NONBLOCK)?;
    Ok(())
}
