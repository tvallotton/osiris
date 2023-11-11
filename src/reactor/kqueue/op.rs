#![allow(warnings)]
#![allow(non_upper_case_globals)]

use std::cell::Cell;
use std::convert::Infallible;
use std::ffi::CString;
use std::io::{Error, Result};
use std::mem::size_of_val;
use std::net::{Shutdown, SocketAddr};
use std::os::fd::{FromRawFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use std::slice;
use std::time::{Duration, Instant};

use libc::{iovec, kevent, msghdr, EVFILT_READ, EVFILT_WRITE, EV_ADD, EV_ENABLE, EV_ONESHOT};

use super::submit;
use crate::buf::{IoBuf, IoBufMut};
use crate::net::utils::{socket_addr, to_std_socket_addr};
use crate::task::spawn_blocking;
use crate::utils::syscall;

pub use super::super::utils::{make_blocking, make_nonblocking, socket};
pub use crate::reactor::nonblocking::*;

const zeroed: libc::kevent = libc::kevent {
    ident: 0,
    filter: 0,
    flags: 0,
    fflags: 0,
    data: 0,
    udata: null_mut(),
};

pub fn read_event(fd: i32) -> kevent {
    let mut event = zeroed;
    event.ident = fd as _;
    event.filter = EVFILT_READ;
    event.flags = EV_ENABLE | EV_ADD | EV_ONESHOT;
    event
}

pub fn write_event(fd: i32) -> kevent {
    let mut event = zeroed;
    event.ident = fd as _;
    event.filter = EVFILT_WRITE;
    event.flags = EV_ENABLE | EV_ADD | EV_ONESHOT;
    event
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

/// Submits a timeout operation to the queue
pub async fn sleep(dur: Duration) -> Result<()> {
    let mut event = zeroed;
    event.ident += event_id();
    event.flags = libc::EV_ADD | EV_ENABLE;
    event.filter = libc::EVFILT_TIMER;
    event.data = dur.as_millis() as _;
    let time = Instant::now();
    submit(event, || {
        if time.elapsed() < dur {
            Err(Error::from_raw_os_error(libc::EAGAIN))
        } else {
            Ok(())
        }
    })
    .await
}
