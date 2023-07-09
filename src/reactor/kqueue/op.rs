#![allow(non_upper_case_globals)]

use std::{future::poll_fn, io::Result, ptr::null_mut};

use libc::EVFILT_READ;

use crate::{buf::IoBufMut, reactor};

macro_rules! syscall {
    ($name: ident, $($args:expr),*) => {{
        let res = unsafe {
            libc::$name($($args),*)
        };
        if res < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(res)
        }

    }};
}

const zeroed: libc::kevent = libc::kevent {
    ident: 0,
    filter: 0,
    flags: 0,
    fflags: 0,
    data: 0,
    udata: null_mut(),
};
async fn submit(_: libc::kevent) {
    todo!()
}

/// Attempts to read from a file descriptor into the buffer
pub async fn read_at<B: IoBufMut>(fd: i32, mut buf: B, pos: i64) -> (Result<usize>, B) {
    let mut event = zeroed;
    event.ident = fd as _;
    event.filter = EVFILT_READ;
    submit(event).await;

    let read = syscall!(read, fd, buf.stable_mut_ptr() as _, buf.bytes_total())?;

    todo!()
}
