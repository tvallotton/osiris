use std::{ffi::CString, io::Result};

use io_uring::{
    opcode::{Close, Fsync, Read, Statx, UnlinkAt, Write},
    types::{Fd, FsyncFlags},
};
use libc::AT_FDCWD;

use crate::buf::{IoBuf, IoBufMut};

use super::submit;

/// Attempts to close a file descriptor
pub async fn close(fd: i32) -> Result<()> {
    let sqe = Close::new(Fd(fd)).build();
    unsafe { submit(sqe, ()) }.await.0.map(|_| ())
}

/// Attempts to read from a file descriptor into the buffer
pub async fn read_at<B: IoBufMut>(fd: i32, mut buf: B, pos: i64) -> (Result<usize>, B) {
    let sqe = Read::new(Fd(fd), buf.stable_mut_ptr(), buf.bytes_total() as _)
        .offset64(pos)
        .build();
    let (cqe, buf) = unsafe { submit(sqe, buf).await };
    (cqe.map(|cqe| cqe.result() as usize), buf)
}

pub async fn write_at<B: IoBuf>(fd: i32, buf: B, pos: i64) -> (Result<usize>, B) {
    let sqe = Write::new(Fd(fd), buf.stable_ptr(), buf.bytes_init() as _)
        .offset64(pos)
        .build();
    let (cqe, buf) = unsafe { submit(sqe, buf).await };
    (cqe.map(|cqe| cqe.result() as usize), buf)
}

/// Performs an fsync call
pub async fn fsync(fd: i32, flags: FsyncFlags) -> Result<i32> {
    let sqe = Fsync::new(Fd(fd)).flags(flags).build();
    // Safety: no resource tracking needed
    let res = unsafe { submit(sqe, ()).await.0?.result() };
    Ok(res)
}
/// removes a file
pub async fn unlink_at(path: CString) -> Result<i32> {
    let sqe = UnlinkAt::new(Fd(AT_FDCWD), path.as_ptr()).build();
    let res = unsafe { submit(sqe, path) };
    Ok(res.await.0?.result())
}

/// performs a statx "system call" on a file or path
pub async fn statx(fd: i32, path: Option<CString>) -> Result<libc::statx> {
    let pathname = path.as_ref().map(|x| x.as_ptr()).unwrap_or(b"\0".as_ptr());

    let statx = std::mem::MaybeUninit::<libc::statx>::uninit();
    let mut statx = Box::new(statx);
    let sqe = Statx::new(Fd(fd), pathname, statx.as_mut_ptr().cast())
        .mask(libc::STATX_ALL)
        .flags(if path.is_none() {
            libc::AT_EMPTY_PATH
        } else {
            0
        })
        .build();
    // Safety: both resources are guarded
    let (res, (_, statx)) = unsafe { submit(sqe, (path, statx)).await };
    // Safety: initialized by io-uring
    res.map(|_| unsafe { statx.assume_init_read() })
}
