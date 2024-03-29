use libc::{iovec, msghdr, AT_FDCWD, AT_SYMLINK_NOFOLLOW};
use submit::submit_once;

use crate::buf::{IoBuf, IoBufMut};
use crate::net::utils::{socket_addr, to_std_socket_addr};
use crate::reactor::op::{make_nonblocking, read_event, write_event};
use crate::task::spawn_blocking;
use crate::utils::{statx, syscall};

use std::ffi::CString;
use std::io::{Error, Result};
use std::mem::{size_of_val, zeroed};
use std::net::{Shutdown, SocketAddr};
use std::os::fd::{FromRawFd, OwnedFd};

use super::submit;

pub async fn fs_read<B: IoBufMut + Send + Sync>(fd: i32, mut buf: B) -> (Result<usize>, B) {
    spawn_blocking(move || {
        let r = syscall!(read, fd, buf.stable_mut_ptr().cast(), buf.bytes_total());
        (r.map(|n| n as usize), buf)
    })
    .await
}

pub async fn fs_write<B: IoBuf + Send + Sync>(fd: i32, buf: B) -> (Result<usize>, B) {
    spawn_blocking(move || {
        let r = syscall!(write, fd, buf.stable_ptr().cast(), buf.bytes_total());
        (r.map(|n| n as usize), buf)
    })
    .await
}

pub async fn mkdir_at(path: CString) -> Result<()> {
    spawn_blocking(move || syscall!(mkdirat, libc::AT_FDCWD, path.as_ptr(), 0o666)).await?;
    Ok(())
}

pub async fn statx(fd: i32, path: Option<CString>, flags: i32) -> Result<statx> {
    let stat = spawn_blocking(move || {
        let mut stat: libc::stat = unsafe { zeroed() };

        match path {
            None if flags != AT_SYMLINK_NOFOLLOW => syscall!(fstat, fd, &mut stat)?,
            Some(path) if flags != AT_SYMLINK_NOFOLLOW => syscall!(stat, path.as_ptr(), &mut stat)?,
            Some(path) => syscall!(lstat, path.as_ptr(), &mut stat)?,
            None => {
                let mut path = [0 as libc::c_char; libc::PATH_MAX as _];
                syscall!(fcntl, fd, libc::F_GETPATH, &mut path)?;
                syscall!(lstat, path.as_ptr(), &mut stat)?
            }
        };

        Result::Ok(stat)
    })
    .await?;
    Ok(statx::from_stat(stat))
}

pub async fn unlink_at(path: CString, flags: i32) -> Result<()> {
    spawn_blocking(move || syscall!(unlinkat, AT_FDCWD, path.as_ptr(), flags)).await?;
    Ok(())
}

pub async fn open_at(path: CString, flags: i32, mode: libc::mode_t) -> Result<i32> {
    spawn_blocking(move || syscall!(openat, AT_FDCWD, path.as_ptr(), flags, mode as u32)).await
}

pub async fn read_at<B: IoBufMut>(fd: i32, mut buf: B, _pos: i64) -> (Result<usize>, B) {
    let res = read_nonblock(fd, buf.stable_mut_ptr(), buf.bytes_total()).await;
    if let Ok(val) = res {
        unsafe { buf.set_init(buf.bytes_init().max(val)) };
    };
    (res, buf)
}

pub async fn write_at<B: IoBuf>(fd: i32, buf: B, _pos: i64) -> (Result<usize>, B) {
    let res = write_nonblock(fd, buf.stable_ptr(), buf.bytes_total()).await;
    (res, buf)
}

pub async fn read_nonblock(fd: i32, buf: *mut u8, len: usize) -> Result<usize> {
    let event = read_event(fd);
    let res = submit(event, || syscall!(read, fd, buf.cast(), len)).await?;
    Ok(res as _)
}

pub async fn recv<B: IoBufMut>(fd: i32, mut buf: B) -> (Result<usize>, B) {
    let event = read_event(fd);
    let res = submit(event, || {
        syscall!(recv, fd, buf.stable_mut_ptr().cast(), buf.bytes_total(), 0)
    })
    .await;
    (res.map(|v| v as _), buf)
}

pub async fn recvfrom<B: IoBufMut>(fd: i32, mut buf: B) -> (Result<(usize, SocketAddr)>, B) {
    let event = read_event(fd);

    let mut sockaddr: libc::sockaddr = unsafe { zeroed() };
    let mut sock_len: libc::socklen_t = size_of_val(&sockaddr) as _;
    let res = submit(event, || {
        syscall!(
            recvfrom,
            fd,
            buf.stable_mut_ptr().cast(),
            buf.bytes_total(),
            0,
            &mut sockaddr,
            &mut sock_len,
        )
    })
    .await;

    let res = res.and_then(|read| {
        let sockaddr = to_std_socket_addr(&sockaddr)?;
        Ok((read as _, sockaddr))
    });
    (res, buf)
}

pub async fn connect(fd: i32, addr: SocketAddr) -> Result<()> {
    let event = write_event(fd);

    let (addr, len) = socket_addr(&addr);
    dbg!();
    submit_once(event, || syscall!(connect, fd, &addr as *const _ as _, len)).await?;

    retrieve_connection_error(fd)?;
    Ok(())
}

fn retrieve_connection_error(fd: i32) -> Result<()> {
    let optval = &mut 0;
    let optlen = &mut size_of_val(optval);
    syscall!(
        getsockopt,
        fd,
        libc::SOL_SOCKET,
        libc::SO_ERROR,
        optval as *mut _ as _,
        optlen as *mut _ as _,
    )
    .ok();
    if *optval != 0 {
        return Err(Error::from_raw_os_error(*optval));
    }
    Ok(())
}

pub async fn accept(fd: i32) -> Result<(OwnedFd, SocketAddr)> {
    let mut address = unsafe { std::mem::zeroed::<libc::sockaddr>() };
    let mut address_len = size_of_val(&address) as u32;
    let event = read_event(fd);

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

    let event = write_event(fd);
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

pub async fn write_nonblock(fd: i32, buf: *const u8, len: usize) -> Result<usize> {
    let event = write_event(fd);
    let res = submit(event, || syscall!(write, fd, buf.cast(), len)).await;
    Ok(res? as usize)
}

pub async fn fsync(fd: i32) -> Result<()> {
    spawn_blocking(move || syscall!(fsync, fd)).await?;
    Ok(())
}

pub async fn symlink(original: CString, link: CString) -> Result<()> {
    spawn_blocking(move || syscall!(symlinkat, original.as_ptr(), libc::AT_FDCWD, link.as_ptr()))
        .await?;
    Ok(())
}
