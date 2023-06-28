use crate::net::socket::{Domain, Protocol, Type};
use crate::net::ToSocketAddrs;
use std::{io::Result, net::SocketAddr};

use super::{socket::Socket, to_socket_addr::try_until_success, TcpStream};

/// A TCP socket server, listening for connections.
///
/// After creating a `TcpListener` by [`bind`]ing it to a socket address, it listens
/// for incoming TCP connections. These can be accepted by calling [`accept`].
///
/// The socket will be closed when the value is dropped.
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
    /// #[osiris::main(scale = true)]
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

    pub async fn accept(&self) -> Result<(TcpStream, SocketAddr)> {
        let (socket, addr) = self.socket.accept().await?;
        Ok((TcpStream { socket }, addr))
    }
}

#[test]
fn reuseport() {
    crate::block_on(async {
        let _listener1 = TcpListener::bind("127.0.0.1:8080".parse().unwrap())
            .await
            .unwrap();
        let _listener2 = TcpListener::bind("127.0.0.1:8080".parse().unwrap())
            .await
            .unwrap();
    })
    .unwrap();
}
