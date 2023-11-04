use crate::buf::{IoBuf, IoBufMut};
use crate::task::spawn_blocking;

use crate::utils::syscall;
use std::io::Result;

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

pub fn make_nonblocking(fd: i32) -> Result<()> {
    let options = syscall!(fcntl, fd, libc::F_GETFL)?;
    syscall!(fcntl, fd, libc::F_SETFL, options | libc::O_NONBLOCK)?;
    Ok(())
}
