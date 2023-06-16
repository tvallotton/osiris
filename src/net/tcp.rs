use std::{
    io::{Error, ErrorKind, Result},
    net::Shutdown,
};

use crate::{
    buf::{IoBuf, IoBufMut},
    reactor::op,
};

use super::{
    socket::{Domain, Protocol, Socket, Type},
    to_socket_addr::{try_until_success, ToSocketAddrs},
};

pub struct TcpStream {
    socket: Socket,
}

impl TcpStream {
    pub async fn connect<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        let socket = try_until_success(addr, |addr| async move {
            let domain = Domain::from(addr);
            let ty = Type::STREAM;
            let proto = Protocol::TCP;
            Socket::new(domain, ty, proto).await
        })
        .await?;
        Ok(TcpStream { socket })
    }
    /// Read some data from the stream into the buffer, returning the original buffer and quantity of data read.
    pub async fn read<B: IoBufMut>(&self, buf: B) -> (Result<usize>, B) {
        op::read_at(self.socket.fd, buf, 0).await
    }
    /// Write some data to the stream from the buffer, returning the original buffer and quantity of data written.
    pub async fn write<B: IoBuf>(&self, buf: B) -> (Result<usize>, B) {
        op::write_at(self.socket.fd, buf, 0).await
    }

    /// Attempts to write an entire buffer to the stream.
    ///
    /// This method will continuously call [`write`] until there is no more data to be
    /// written or an error is returned. This method will not return until the entire
    /// buffer has been successfully written or an error has occurred.
    ///
    /// If the buffer contains no data, this will never call [`write`].
    ///
    /// # Errors
    ///
    /// This function will return the first error that [`write`] returns.
    ///
    /// [`write`]: Self::write
    pub async fn write_all<B: IoBuf>(&self, mut buf: B) -> (Result<()>, B) {
        let mut n = 0;
        while n < buf.bytes_init() {
            let (written, buf_) = self.write(buf.slice(n..)).await;
            buf = buf_.into_inner();
            match written {
                Ok(0) => {
                    return (
                        Err(Error::new(
                            ErrorKind::WriteZero,
                            "failed to write whole buffer",
                        )),
                        buf,
                    )
                }
                Ok(written) => n += written,
                Err(err) => return (Err(err), buf),
            }
        }
        (Ok(()), buf)
    }
    // Shuts down the read, write, or both halves of this connection.
    // This function will cause all pending and future I/O on the specified portions to return immediately with an appropriate value.
    pub async fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.socket.shutdown(how).await
    }
}
