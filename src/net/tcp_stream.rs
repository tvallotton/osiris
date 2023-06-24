use std::io::{Error, ErrorKind, Result};
use std::net::Shutdown;

use crate::buf::{IoBuf, IoBufMut};
use crate::reactor::op;

use super::socket::{Domain, Protocol, Socket, Type};
use super::to_socket_addr::{try_until_success, ToSocketAddrs};

/// A TCP stream between a local and a remote socket.
///
/// After creating a `TcpStream` by either [`connect`]ing to a remote host or
/// [`accept`]ing a connection on a [`TcpListener`], data can be transmitted
/// by [reading] and [writing] to it.
///
/// If `close` is not called before dropping the file, the file is closed in
/// the background, but there is no guarantee as to **when** the close
/// operation will complete, and if any errors occur they will be discarded.
/// Note that letting a file be closed in the background incurs in an additional
/// allocation. The reading and writing portions of the connection can also be
/// shut down individually with the [`shutdown`] method.
///
/// The Transmission Control Protocol is specified in [IETF RFC 793].
///
///
/// # Example
/// A simple HTTP request to www.example.com.
/// ```
/// use osiris::net::TcpStream;
///
/// #[osiris::main]
/// async fn main() -> std::io::Result<()> {
///     let stream = TcpStream::connect("www.example.com:80").await?;
///     stream
///         .write_all(b"GET / HTTP/1.1\r\nHost: www.example.com\r\n\r\n")
///         .await.0?;
///     let buf = vec![0; 256];
///     let (n, buf) = stream.read(buf).await;
///     let response = &buf[..n?];
///     # dbg!(std::str::from_utf8(response));
///     assert!(response .starts_with(b"HTTP/1.1 200 OK"));
///     stream.close().await?;
///     Ok(())
/// }
/// ```
/// [`TcpListener`]: super::TcpListener
/// [`accept`]: super::TcpListener::accept
/// [`connect`]: TcpStream::connect
/// [IETF RFC 793]: https://tools.ietf.org/html/rfc793
/// [reading]: TcpStream::read
/// [`shutdown`]: TcpStream::shutdown
/// [writing]: TcpStream::write
pub struct TcpStream {
    pub(crate) socket: Socket,
}

impl TcpStream {
    /// Opens a TCP connection to a remote host.
    ///
    /// `addr` is an address of the remote host. Anything which implements
    /// [`ToSocketAddrs`] trait can be supplied for the address; see this trait
    /// documentation for concrete examples.
    ///
    /// If `addr` yields multiple addresses, `connect` will be attempted with
    /// each of the addresses until a connection is successful. If none of
    /// the addresses result in a successful connection, the error returned from
    /// the last connection attempt (the last address) is returned.
    ///
    /// # Examples
    ///
    /// Open a TCP connection to `127.0.0.1:8080`:
    ///
    /// ```
    /// use osiris::net::TcpStream;
    ///
    /// #[osiris::main]
    /// async fn main() -> std::io::Result<()> {
    ///     if let Ok(stream) = TcpStream::connect("127.0.0.1:8080").await {
    ///         println!("Connected to the server!");
    ///     } else {
    ///         println!("Couldn't connect to server...");
    ///     }
    ///     Ok(())
    /// }
    /// ```
    ///
    /// Open a TCP connection to `127.0.0.1:8080`. If the connection fails, open
    /// a TCP connection to `127.0.0.1:8081`:
    ///
    /// ```no_run
    /// use osiris::net::{SocketAddr, TcpStream};
    ///
    /// #[osiris::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let addrs = [
    ///         SocketAddr::from(([127, 0, 0, 1], 8080)),
    ///         SocketAddr::from(([127, 0, 0, 1], 8081)),
    ///     ];
    ///
    ///     let stream = TcpStream::connect(&addrs[..]).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn connect<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        let socket = try_until_success(addr, |addr| async move {
            let domain = Domain::from(addr);
            let ty = Type::STREAM;
            let proto = Protocol::TCP;
            let socket = Socket::new(domain, ty, proto)?;
            socket.connect(addr).await?;
            Ok(socket)
        })
        .await?;
        Ok(TcpStream { socket })
    }
    /// Read some data from the stream into the buffer, returning the original buffer and quantity of data read.
    ///
    /// # Example
    /// ```no_run
    /// use osiris::net::{SocketAddr, TcpStream};
    ///
    /// #[osiris::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let stream = TcpStream::connect("127.0.0.1:8080").await?;
    ///     let buf = vec![0; 128];
    ///     let (n, buf) = stream.read(buf).await;
    ///     let read = &buf[..n?];
    ///     Ok(())
    /// }
    /// ```
    pub async fn read<B: IoBufMut>(&self, buf: B) -> (Result<usize>, B) {
        op::read_at(self.socket.fd, buf, 0).await
    }
    /// Write some data to the stream from the buffer, returning the original buffer and quantity of data written.
    ///
    /// # Example
    /// ```no_run
    /// use osiris::net::{SocketAddr, TcpStream};
    ///
    /// #[osiris::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let stream = TcpStream::connect("127.0.0.1:8080").await?;
    ///     let message = "some message";
    ///     let (n, _) = stream.write("some message").await;
    ///     assert_eq!(message.len(), n?);
    ///     Ok(())
    /// }
    /// ```
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
    /// # Examples
    /// ```no_run
    /// use osiris::net::TcpStream;
    ///
    /// #[osiris::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let stream = TcpStream::connect("127.0.0.1:8080").await?;
    ///     let (res, _) = stream.write_all("GET /index.html HTTP/1.0\r\n\r\n").await;
    ///     res?;
    ///     Ok(())
    /// }
    /// ```
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

    /// Shuts down the read, write, or both halves of this connection.
    ///
    /// This function will cause all pending and future I/O on the specified
    /// portions to return immediately with an appropriate value (see the
    /// documentation of [`Shutdown`]).
    ///
    /// # Platform-specific behavior
    ///
    /// Calling this function multiple times may result in different behavior,
    /// depending on the operating system. On Linux, the second call will
    /// return `Ok(())`, but on macOS, it will return `ErrorKind::NotConnected`.
    /// This may change in the future.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use osiris::net::{Shutdown, TcpStream};
    ///
    /// #[osiris::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let stream = TcpStream::connect("127.0.0.1:8080").await?;
    ///     stream.shutdown(Shutdown::Both).await?;
    ///     stream.close().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.socket.shutdown(how).await
    }
    /// Closes the file descriptor. Calling this method is recommended
    /// over letting the value be dropped.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use osiris::net::{Shutdown, TcpStream};
    ///
    /// #[osiris::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let stream = TcpStream::connect("127.0.0.1:8080").await?;
    ///     stream.shutdown(Shutdown::Both).await?;
    ///     stream.close().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn close(self) -> Result<()> {
        self.socket.close().await
    }
}

async fn foo() {
    let stream = TcpStream::connect("asd").await.unwrap();
    stream.read(vec![]).await.0.unwrap();
}
