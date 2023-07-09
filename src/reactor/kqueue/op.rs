#![allow(warnings)]
#![allow(non_upper_case_globals)]

use std::cell::Cell;
use std::io::{Error, Result};
use std::ptr::null_mut;
use std::time::{Duration, Instant};

use libc::{EVFILT_READ, EVFILT_WRITE};

use crate::buf::{IoBuf, IoBufMut};
use crate::reactor::kqueue::event::submit;

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

/// Attempts to read from a file descriptor into the buffer
pub async fn read_at<B: IoBufMut>(fd: i32, mut buf: B, _pos: i64) -> (Result<usize>, B) {
    let mut event = zeroed;
    event.ident = fd as _;
    event.filter = EVFILT_READ;

    let res = submit(event, || {
        syscall!(read, fd, buf.stable_mut_ptr() as _, buf.bytes_total())
    })
    .await;
    (res.map(|len| len as _), buf)
}

/// Attempts to read from a file descriptor into the buffer
pub async fn write_at<B: IoBuf>(fd: i32, buf: B, _pos: i64) -> (Result<usize>, B) {
    let mut event = zeroed;
    event.ident = fd as _;
    event.filter = EVFILT_WRITE;

    let res = submit(event, || {
        syscall!(write, fd, buf.stable_ptr() as _, buf.bytes_init())
    })
    .await;
    (res.map(|len| len as _), buf)
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
pub async fn timeout(dur: Duration) -> Result<()> {
    let mut event = zeroed;
    event.ident += event_id();
    event.flags = libc::EV_ADD;
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

#[test]
fn foo() {
    use std::time::Instant;
    crate::block_on(async {
        let time = Instant::now();
        timeout(Duration::from_secs(1)).await.unwrap();
        dbg!(time.elapsed());
    })
    .unwrap();
}
