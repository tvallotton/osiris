use super::{socket, socket_addr};
use crate::detach;
use crate::shared_driver::submit;
use io_uring::opcode::{Close, Shutdown};
use io_uring::{opcode::Connect, types::Fd};
use libc::SOCK_STREAM;
use std::io::{self, Error, ErrorKind, Result};
use std::mem::forget;
use std::net::SocketAddr;
use std::net::{self, Shutdown::*};

use super::to_socket_addr::ToSocketAddrs;

struct TcpStream {
    fd: i32,
}

fn invalid_input<T>() -> Result<T> {
    Err(Error::new(
        ErrorKind::InvalidInput,
        "could not resolve to any addresses",
    ))
}

impl TcpStream {
    /// Open a TCP connection to a remote host.
    pub async fn connect<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        let addresses = addr.to_socket_addrs().await?;
        let mut last_error = None;
        for addr in addresses {
            let result = Self::connect_addr(addr).await;
            if result.is_ok() {
                return result;
            } else {
                last_error = Some(result);
            }
        }
        last_error.unwrap_or_else(invalid_input)
    }

    /// Establishe a connection to the specified `addr`.
    pub async fn connect_addr(addr: SocketAddr) -> io::Result<Self> {
        let fd = socket(addr, SOCK_STREAM)?;
        let (addr, addrlen) = socket_addr(&addr);
        let addr = Box::new(addr);
        let sqe = Connect::new(Fd(fd), &addr as _ as _, addrlen).build();
        let (res, addr) = unsafe { submit(sqe, addr).await };
        let cqe = res?;
        if cqe.result() < 0 {
            return Err(Error::from_raw_os_error(-cqe.result()));
        }
        let stream = TcpStream { fd };
        Ok(stream)
    }

    /// Shutdowns the socket
    pub async fn shutdown(&self, how: net::Shutdown) -> Result<()> {
        let how = match how {
            Read => libc::SHUT_RD,
            Write => libc::SHUT_WR,
            Both => libc::SHUT_RDWR,
        };
        let sqe = Shutdown::new(Fd(self.fd), how).build();
        let cqe = unsafe { submit(sqe, ()).await.0? };
        if cqe.result() > 0 {
            return Ok(());
        }
        Err(Error::from_raw_os_error(-cqe.result()))
    }

    /// Closes the file descriptor
    pub async fn close(self) -> Result<()> {
        let sqe = Close::new(Fd(self.fd)).build();
        unsafe { submit(sqe, ()).await.0? };
        Ok(())
    }
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        let fd = self.fd;
        detach(async {
            let s = TcpStream { fd };
            s.shutdown(Write);
            s.close().await;
            forget(s);
            todo!("must read before close");
        });
    }
}
