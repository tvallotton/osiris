use std::{
    io::Result,
    os::fd::{AsRawFd, OwnedFd},
};

use crate::utils::syscall;

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
