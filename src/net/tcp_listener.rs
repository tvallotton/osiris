use crate::net::socket::{Domain, Protocol, Type};
use crate::net::ToSocketAddrs;
use std::io::Result;
use std::net::SocketAddr;
use std::os::fd::{FromRawFd, IntoRawFd};

use super::socket::Socket;
use super::to_socket_addr::try_until_success;
use super::TcpStream;

/// A TCP socket server, listening for connections.
///
/// After creating a `TcpListener` by [`bind`]ing it to a socket address, it listens
/// for incoming TCP connections. These can be accepted by calling [`accept`].
///
/// The socket will be closed asynchronously on the background when the value is dropped.
/// There is no guarantee of when the operation will succeed. Alternatively, the listener
/// can be closed explicitly with the [`close`](TcpListener::close) method.
///
/// The Transmission Control Protocol is specified in [IETF RFC 793].
///
/// [`accept`]: TcpListener::accept
/// [`bind`]: TcpListener::bind
/// [IETF RFC 793]: https://tools.ietf.org/html/rfc793
///
/// # Examples
/// A multithreaded tcp echo server.
/// ```no_run
/// use osiris::buf::IoBuf;
/// use osiris::detach;
/// use osiris::net::{TcpListener, TcpStream};
/// use std::io::Result;
///
/// async fn handle_client(stream: TcpStream) -> Result<()> {
///     let buf = vec![0; 1048];
///     let (n, buf) = stream.read(buf).await;
///     let buf = buf.slice(..n?);
///     stream.write_all(buf).await.0?;
///     stream.close().await
/// }
///
/// #[osiris::main(scale = true)]
/// async fn main() -> Result<()> {
///     let listener = TcpListener::bind("127.0.0.1:8080").await?;
///     loop {
///         let (stream, _) = listener.accept().await.unwrap();
///         detach(handle_client(stream));
///     }
///     Ok(())
/// }
/// ```
pub struct TcpListener {
    socket: Socket,
}

impl TcpListener {
    /// Creates a new `TcpListener` which will be bound to the specified
    /// address.
    ///
    /// The returned listener is ready for accepting connections.
    ///
    /// Binding with a port number of 0 will request that the OS assigns a port
    /// to this listener. The port allocated can be queried via the
    /// [`TcpListener::local_addr`] method.
    ///
    /// The address type can be any implementor of [`ToSocketAddrs`] trait. See
    /// its documentation for concrete examples.
    ///
    /// If `addr` yields multiple addresses, `bind` will be attempted with
    /// each of the addresses until one succeeds and returns the listener. If
    /// none of the addresses succeed in creating a listener, the error returned
    /// from the last attempt (the last address) is returned.
    ///
    /// # Examples
    ///
    /// Creates a TCP listener bound to `127.0.0.1:80`:
    ///
    /// ```no_run
    /// use osiris::net::TcpListener;
    ///
    /// #[osiris::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let listener = TcpListener::bind("127.0.0.1:80").await?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// Creates a TCP listener bound to `127.0.0.1:80`. If that fails, create a
    /// TCP listener bound to `127.0.0.1:443`:
    ///
    /// ```no_run
    /// use osiris::net::{SocketAddr, TcpListener};
    ///
    /// #[osiris::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let addrs = [
    ///         SocketAddr::from(([127, 0, 0, 1], 80)),
    ///         SocketAddr::from(([127, 0, 0, 1], 443)),
    ///     ];
    ///     let listener = TcpListener::bind(&addrs[..]).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> Result<TcpListener> {
        try_until_success(addr, |addr| async move {
            let domain = Domain::from(addr);
            let socket = Socket::new(domain, Type::STREAM, Protocol::TCP)?;
            socket.set_reuseport()?;
            socket.bind(&addr)?;
            socket.listen(8192)?;
            Ok(TcpListener { socket })
        })
        .await
    }

    /// Accept a new incoming connection from this listener.
    ///
    /// This function will block the calling thread until a new TCP connection
    /// is established. When established, the corresponding [`TcpStream`] and the
    /// remote peer's address will be returned.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use osiris::net::TcpListener;
    ///
    /// #[osiris::main(scale = true)]
    /// async fn main() -> std::io::Result<()> {
    ///     let listener = TcpListener::bind("127.0.0.1:8080").await?;
    ///     match listener.accept().await {
    ///         Ok((_socket, addr)) => println!("new client: {addr:?}"),
    ///         Err(e) => println!("couldn't get client: {e:?}"),
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn accept(&self) -> Result<(TcpStream, SocketAddr)> {
        let (socket, addr) = self.socket.accept().await?;
        Ok((TcpStream { socket }, addr))
    }
    /// Closes the file descriptor. Calling this method is recommended
    /// over letting the value be dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use osiris::net::{Shutdown, TcpListener};
    ///
    /// #[osiris::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let listener = TcpListener::bind("127.0.0.1:8090").await?;
    ///     listener.close().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn close(self) -> Result<()> {
        self.socket.close().await
    }

    /// Turns a [`osiris::net::TcpListener`](TcpListener) into a [`std::net::TcpListener`].
    ///
    /// It is unspecified whether the returned [`std::net::TcpListener`] will be
    /// set as nonblocking or not. Whichever behavior will be used should be set
    /// after the operation.
    ///
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::error::Error;
    ///
    /// #[osiris::main]
    /// async fn main() -> Result<(), Box<dyn Error>> {
    ///     let osiris_listener = osiris::net::TcpListener::bind("127.0.0.1:0").await?;
    ///     let std_listener = osiris_listener.into_std();
    ///     std_listener.set_nonblocking(false)?;
    ///     Ok(())
    /// }
    /// ```
    pub fn into_std(self) -> std::net::TcpListener {
        let fd = self.into_raw_fd();
        unsafe { std::net::TcpListener::from_raw_fd(fd) }
    }

    /// Creates a new `TcpListener` from a `std::net::TcpListener`.
    ///
    /// This function is intended to be used to wrap a TCP listener from the
    /// standard library in the Osiris equivalent.
    ///
    /// This API can be used with `socket2` or `libc::socket` to customize
    /// a socket before it is used. Alternatively, the
    /// [`from_raw_fd`](FromRawFd::from_raw_fd) method can also be used
    /// to create an osiris listener.
    pub fn from_std(listener: std::net::TcpListener) -> Self {
        let fd = listener.into_raw_fd();
        let socket = Socket { fd };
        Self { socket }
    }

    /// Returns the local address that this listener is bound to.
    pub fn local_addr(&self) -> Result<()> {
        todo!()
    }
}

impl FromRawFd for TcpListener {
    unsafe fn from_raw_fd(fd: std::os::fd::RawFd) -> Self {
        TcpListener {
            socket: Socket::from_raw_fd(fd),
        }
    }
}

impl IntoRawFd for TcpListener {
    fn into_raw_fd(self) -> std::os::fd::RawFd {
        self.socket.into_raw_fd()
    }
}

#[test]
fn reuseport() {
    crate::block_on(async {
        let _listener1 = TcpListener::bind("127.0.0.1:8080").await.unwrap();
        let _listener2 = TcpListener::bind("127.0.0.1:8080").await.unwrap();
        _listener1.close().await.unwrap();
    })
    .unwrap();
}

#[test]
fn accept() {
    crate::block_on(async {
        let request = [0u8; 32].map(|_| fastrand::u8(..)).to_vec();
        let response = [0u8; 32].map(|_| fastrand::u8(..)).to_vec();
        crate::detach({
            let request = request.clone();
            let response = response.clone();
            async move {
                let listener = TcpListener::bind("127.0.0.1:8083").await.unwrap();
                let (stream, _) = listener.accept().await.unwrap();
                let buf = vec![0u8; 32];
                let (n, buf) = stream.read(buf).await;
                assert_eq!(&buf[..n.unwrap()], &request.clone()[..]);
                stream.write(response).await.0.unwrap();
                stream.shutdown(std::net::Shutdown::Write).await.unwrap();
                stream.shutdown(std::net::Shutdown::Both).await.unwrap();
                stream.close().await.unwrap();
                listener.close().await.unwrap();
            }
        });

        crate::detach({
            let request = request.clone();
            let response = response.clone();
            async move {
                let stream = TcpStream::connect("127.0.0.1:8083").await.unwrap();
                stream.write_all(request).await.0.unwrap();
                let buf = vec![0u8; 32];
                let (n, buf) = stream.read(buf).await;
                assert_eq!(&buf[..n.unwrap()], &response[..]);
                stream.shutdown(std::net::Shutdown::Read).await.unwrap();
                stream.close().await.unwrap();
            }
        })
        .await
    })
    .unwrap();
}
