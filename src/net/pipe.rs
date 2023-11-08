#![allow(warnings)]
use std::io::Error;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

// use io_uring::types::Fd;

use crate::buf::{IoBuf, IoBufMut};

use crate::reactor::op::{self, write_nonblock};
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

    let sender = Sender {
        fd: unsafe { OwnedFd::from_raw_fd(fds[1]) },
    };

    let receiver = Receiver {
        fd: unsafe { OwnedFd::from_raw_fd(fds[0]) },
    };

    op::make_nonblocking(&sender.fd)?;
    op::make_nonblocking(&receiver.fd)?;
    Ok((sender, receiver))
}

impl Sender {
    pub async fn write<B: IoBuf>(&self, buf: B) -> (Result<usize, Error>, B) {
        let fd = self.fd.as_raw_fd();
        op::write_at(fd, buf, -1).await
    }

    pub async fn write_nonblock(&self, buf: &[u8]) -> Result<usize, Error> {
        let fd = self.fd.as_raw_fd();
        let out = write_nonblock(fd, buf.as_ptr(), buf.len()).await;
        out
    }

    pub fn write_block(&self, buf: &[u8]) -> Result<usize, Error> {
        op::make_blocking(&self.fd)?;
        let fd = self.fd.as_raw_fd();
        let len = buf.len();
        let buf = buf.as_ptr().cast();
        let res = syscall!(write, fd, buf, len).map(|c| c as _);
        op::make_nonblocking(&self.fd);
        res
    }
}

impl Receiver {
    pub async fn read<B: IoBufMut>(&self, buf: B) -> (Result<usize, Error>, B) {
        let fd = self.fd.as_raw_fd();
        op::read_at(fd, buf, -1).await
    }

    pub async fn read_nonblock(&self, buf: &mut [u8]) -> Result<usize, Error> {
        let fd = self.fd.as_raw_fd();
        let len = buf.len();
        let buf = buf.as_mut_ptr();
        op::read_nonblock(fd, buf, len).await
    }
}

#[test]
fn pipe_smoke_test() {
    println!("Started 10");
    crate::block_on(async {
        println!("pipe()");
        let (tx, rx) = pipe().unwrap();
        let buf = vec![1, 2, 3];
        println!("writing");
        let (res, _) = tx.write(buf).await;
        res.unwrap();
        println!("reading");
        let (res, buf) = rx.read(vec![0, 0, 0]).await;
        res.unwrap();
        assert_eq!(buf, vec![1, 2, 3]);
    })
    .unwrap();
}
