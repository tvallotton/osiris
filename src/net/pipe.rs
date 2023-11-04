#![allow(warnings)]
use std::io::Error;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

// use io_uring::types::Fd;

use crate::buf::{IoBuf, IoBufMut};

use crate::reactor::op;
use crate::utils::syscall;

pub(crate) struct Sender {
    fd: OwnedFd,
}
pub(crate) struct Receiver {
    fd: OwnedFd,
}

pub(crate) fn pipe() -> Result<(Sender, Receiver), Error> {
    let mut fds = [-1, -1];

    syscall!(pipe, &mut fds[0])?;

    let options = syscall!(fcntl, fds[0], libc::F_GETFL)?;
    op::make_nonblocking(fds[0])?;
    op::make_nonblocking(fds[1])?;

    let sender = Sender {
        fd: unsafe { OwnedFd::from_raw_fd(fds[1]) },
    };
    let receiver = Receiver {
        fd: unsafe { OwnedFd::from_raw_fd(fds[0]) },
    };

    Ok((sender, receiver))
}

impl Sender {
    pub async fn write<B: IoBuf>(&self, buf: B) -> (Result<usize, Error>, B) {
        let fd = self.fd.as_raw_fd();
        op::write_at(fd, buf, -1).await
    }

    pub async fn write_nonblock(&self, buf: &[u8]) -> Result<usize, Error> {
        let fd = self.fd.as_raw_fd();
        op::write_nonblock(fd, buf.as_ptr(), buf.len()).await
    }

    pub fn write_block(&self, buf: &[u8]) -> Result<usize, Error> {
        let fd = self.fd.as_raw_fd();
        let len = buf.len();
        let buf = buf.as_ptr().cast();
        syscall!(write, fd, buf, len).map(|c| c as _)
    }
}

impl Receiver {
    pub async fn read<B: IoBufMut>(&self, buf: B) -> (Result<usize, Error>, B) {
        let fd = self.fd.as_raw_fd();
        op::read_at(fd, buf, -1).await
    }
}

#[test]
fn foo() {
    crate::block_on(async {
        let (tx, rx) = pipe().unwrap();
        let buf = vec![1, 2, 3];
        let (res, _) = tx.write(buf).await;
        res.unwrap();
        let (res, buf) = rx.read(vec![0, 0, 0]).await;
        res.unwrap();
        assert_eq!(buf, vec![1, 2, 3]);
    })
    .unwrap();
}
