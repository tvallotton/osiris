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
pub use crate::reactor::nonblocking::*;
use crate::reactor::nonblocking::{submit, submit_once};
pub use crate::reactor::utils::*;
use crate::task::spawn_blocking;
use crate::utils::syscall;

const zeroed: pollfd = pollfd {
    fd: 0,
    events: 0,
    revents: 0,
};

pub fn read_event(fd: i32) -> pollfd {
    pollfd {
        fd,
        events: POLLIN,
        revents: 0,
    }
}

pub fn write_event(fd: i32) -> pollfd {
    pollfd {
        fd,
        events: POLLOUT,
        revents: 0,
    }
}

pub async fn fdatasync(fd: i32) -> Result<()> {
    spawn_blocking(move || syscall!(fdatasync, fd)).await?;
    Ok(())
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
