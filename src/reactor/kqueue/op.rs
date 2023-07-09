#![allow(warnings)]
#![allow(non_upper_case_globals)]

use std::cell::Cell;
use std::convert::Infallible;
use std::io::{Error, Result};

use std::mem::size_of_val;
use std::net::{Shutdown, SocketAddr};
use std::ptr::null_mut;
use std::time::{Duration, Instant};

use libc::{iovec, kevent, msghdr, EVFILT_READ, EVFILT_WRITE, EV_ADD, EV_ENABLE, EV_ONESHOT};

use crate::buf::{IoBuf, IoBufMut};
use crate::net::utils::{socket_addr, to_std_socket_addr};
use crate::reactor::kqueue::event::{submit, submit_once};

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

pub fn socket(domain: i32, ty: i32, proto: i32, _: Option<Infallible>) -> Result<i32> {
    let fd = unsafe { libc::socket(domain as _, ty as i32, proto as _) };
    let flags = syscall!(fcntl, fd, libc::F_GETFL)?;
    let flags = syscall!(fcntl, fd, libc::F_SETFL, flags | libc::O_NONBLOCK)?;
    Ok(fd)
}

/// Attempts to read from a file descriptor into the buffer
pub async fn read_at<B: IoBufMut>(fd: i32, mut buf: B, _pos: i64) -> (Result<usize>, B) {
    let mut event = zeroed;
    event.ident = fd as _;
    event.filter = EVFILT_READ;
    event.flags = EV_ENABLE | EV_ADD;

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
    event.flags = EV_ENABLE | EV_ADD;

    let res = submit(event, || {
        syscall!(write, fd, buf.stable_ptr() as _, buf.bytes_init())
    })
    .await;
    (res.map(|len| len as _), buf)
}

pub async fn recv<B: IoBufMut>(fd: i32, mut buf: B) -> (Result<usize>, B) {
    let mut event = zeroed;
    event.ident = fd as _;
    event.filter = EVFILT_READ;
    event.flags = EV_ADD | EV_ENABLE;

    let res = submit(event, || {
        syscall!(recv, fd, buf.stable_mut_ptr().cast(), buf.bytes_total(), 0)
    })
    .await;
    (res.map(|v| v as _), buf)
}

pub async fn connect(fd: i32, addr: SocketAddr) -> Result<()> {
    let mut kevent = kevent {
        ident: fd as _,
        filter: EVFILT_WRITE,
        flags: EV_ADD | EV_ONESHOT,
        ..zeroed
    };
    dbg!(addr);
    let (addr, len) = socket_addr(&addr);
    submit_once(kevent, || {
        syscall!(connect, fd, &addr as *const _ as _, len)
    })
    .await?;
    Ok(())
}

pub async fn accept(fd: i32) -> Result<(i32, SocketAddr)> {
    let mut address = unsafe { std::mem::zeroed::<libc::sockaddr>() };
    let mut address_len = size_of_val(&address) as u32;
    let mut kevent = zeroed;
    kevent.ident = fd as _;
    kevent.flags = EV_ENABLE | EV_ADD;
    kevent.filter = EVFILT_READ;

    let fd = submit(kevent, || {
        syscall!(accept, fd, &mut address, &mut address_len)
    })
    .await?;
    dbg!(
        fd,
        address_len,
        address.sa_len,
        address.sa_family,
        address.sa_data
    );
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

    let mut kevent = zeroed;
    kevent.ident = fd as _;
    kevent.flags = EV_ADD | EV_ENABLE;
    kevent.filter = EVFILT_WRITE;
    let res = submit(kevent, || syscall!(sendmsg, fd, &msghdr, 0))
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
/// Submits a timeout operation to the queue
pub async fn sleep(dur: Duration) {
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
    .unwrap()
}
