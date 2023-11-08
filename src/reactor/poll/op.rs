#![allow(warnings)]
#![allow(non_upper_case_globals)]

use std::cell::Cell;
use std::convert::Infallible;
use std::ffi::CString;
use std::io::{Error, Result};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

use libc::{iovec, msghdr, pollfd, CLOCK_BOOTTIME, POLLIN, POLLOUT};
use std::mem::size_of_val;
use std::net::{Shutdown, SocketAddr};
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use std::slice;
use std::time::{Duration, Instant};

use crate::buf::{IoBuf, IoBufMut};
use crate::net::utils::{socket_addr, to_std_socket_addr};
use crate::reactor::nonblocking::{submit, submit_once};
use crate::task::spawn_blocking;
use crate::utils::syscall;

pub use crate::reactor::utils::*;

const zeroed: pollfd = pollfd {
    fd: 0,
    events: 0,
    revents: 0,
};

/// Attempts to read from a file descriptor into the buffer
pub async fn read_at<B: IoBufMut>(fd: i32, mut buf: B, _pos: i64) -> (Result<usize>, B) {
    let result = read_nonblock(fd, buf.stable_mut_ptr(), buf.bytes_total())
        .await
        .and_then(|read| unsafe {
            buf.set_init(read.max(buf.bytes_init()));
            Ok(read)
        });
    (result, buf)
}

/// Attempts to read from a file descriptor into the buffer
pub async fn write_at<B: IoBuf>(fd: i32, buf: B, _pos: i64) -> (Result<usize>, B) {
    let size = write_nonblock(fd, buf.stable_ptr(), buf.bytes_init()).await;
    (size, buf)
}

pub async fn recv<B: IoBufMut>(fd: i32, mut buf: B) -> (Result<usize>, B) {
    let mut event = pollfd {
        fd,
        events: POLLIN,
        revents: 0,
    };

    let res = submit(event, || {
        syscall!(recv, fd, buf.stable_mut_ptr().cast(), buf.bytes_total(), 0)
    })
    .await;
    (res.map(|v| v as _), buf)
}

pub async fn connect(fd: i32, addr: SocketAddr) -> Result<()> {
    let mut event = pollfd {
        fd,
        events: POLLOUT,
        revents: 0,
    };

    let (addr, len) = socket_addr(&addr);
    submit_once(event, || syscall!(connect, fd, &addr as *const _ as _, len)).await?;
    let ref mut optval = 0;
    let ref mut optlen = size_of_val(optval);
    syscall!(
        getsockopt,
        fd,
        libc::SOL_SOCKET,
        libc::SO_ERROR,
        optval as *mut _ as _,
        optlen as *mut _ as _,
    );
    if *optval != 0 {
        return Err(Error::from_raw_os_error(*optval));
    }
    Ok(())
}

pub async fn accept(fd: i32) -> Result<(OwnedFd, SocketAddr)> {
    let mut address = unsafe { std::mem::zeroed::<libc::sockaddr>() };
    let mut address_len = size_of_val(&address) as u32;
    let event = pollfd {
        fd,
        events: POLLIN,
        revents: 0,
    };

    let fd = submit(event, || {
        syscall!(accept, fd, &mut address, &mut address_len)
    })
    .await?;

    let fd = unsafe { OwnedFd::from_raw_fd(fd) };

    make_nonblocking(&fd)?;

    let address = to_std_socket_addr(&address)?;
    Ok((fd, address))
}

pub async fn send_to<B: IoBuf>(fd: i32, buf: B, addr: SocketAddr) -> (Result<usize>, B) {
    let mut msghdr: msghdr = unsafe { std::mem::zeroed() };

    // we define the iovec from the buffer
    let mut msg_iov = iovec {
        iov_base: buf.stable_ptr().cast_mut().cast(),
        iov_len: buf.bytes_init(),
    };
    // we set the iovec
    msghdr.msg_iov = &mut msg_iov;
    msghdr.msg_iovlen = 1;

    // we set the address
    let (mut addr, len) = socket_addr(&addr);
    msghdr.msg_name = &mut addr as *mut _ as *mut _;
    msghdr.msg_namelen = len;

    let event = pollfd {
        fd,
        events: POLLOUT,
        revents: 0,
    };
    let res = submit(event, || syscall!(sendmsg, fd, &msghdr, 0))
        .await
        .map(|s| s as _);
    (res, buf)
}

pub async fn close(fd: i32) -> Result<()> {
    syscall!(close, fd).map(|_| ())
}

pub async fn shutdown(fd: i32, how: Shutdown) -> Result<()> {
    let how = match how {
        Shutdown::Read => libc::SHUT_RD,
        Shutdown::Write => libc::SHUT_WR,
        Shutdown::Both => libc::SHUT_RDWR,
    };
    syscall!(shutdown, fd, how).map(|_| ())
}

thread_local! {
    static EVENT_ID: Cell<usize> = Cell::default();
}

fn event_id() -> usize {
    EVENT_ID.with(|cell| {
        let value = cell.get();
        cell.set(value + 1);
        value
    })
}

pub async fn read_nonblock(fd: i32, buf: *mut u8, len: usize) -> Result<usize> {
    let mut event = pollfd {
        fd,
        events: POLLIN,
        revents: 0,
    };
    let res = submit(event, || syscall!(read, fd, buf.cast(), len)).await;
    Ok(res? as usize)
}

pub async fn write_nonblock(fd: i32, buf: *const u8, len: usize) -> Result<usize> {
    let mut event = pollfd {
        fd,
        events: POLLOUT,
        revents: 0,
    };
    let res = submit(event, || syscall!(write, fd, buf.cast(), len)).await?;
    Ok(res as usize)
}

pub async fn symlink(original: impl Into<PathBuf>, link: impl Into<PathBuf>) -> Result<()> {
    let original: PathBuf = original.into();
    let link: PathBuf = link.into();
    spawn_blocking(move || std::os::unix::fs::symlink(original, link)).await
}

pub async fn sleep(dur: Duration) -> Result<()> {
    let mut event = zeroed;

    let fd = syscall!(timerfd_create, CLOCK_BOOTTIME, libc::TFD_NONBLOCK)?;

    let event = pollfd {
        fd,
        events: POLLIN,
        revents: 0,
    };

    let expiration = libc::itimerspec {
        it_value: libc::timespec {
            tv_sec: dur.as_secs() as _,
            tv_nsec: dur.subsec_nanos() as _,
        },
        it_interval: unsafe { std::mem::zeroed() },
    };

    syscall!(timerfd_settime, fd, 0, &expiration, null_mut())?;

    let ref mut buf = [0u8; 8];
    submit(event, || syscall!(read, fd, buf.as_mut_ptr().cast(), 8)).await?;

    Ok(())
}
