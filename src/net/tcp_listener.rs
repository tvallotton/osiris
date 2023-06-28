use std::io::Result;
use std::net::SocketAddr;

use crate::net::socket::{Domain, Protocol, Type};

use super::socket::Socket;
use super::to_socket_addr::{try_until_success, ToSocketAddrs};
use super::TcpStream;

/// A TCP socket server, listening for connections.
///
/// After creating a `TcpListener` by [`bind`]ing it to a socket address, it listens
/// for incoming TCP connections. These can be accepted by calling [`accept`] or by
/// iterating over the [`Incoming`] iterator returned by [`incoming`][`TcpListener::incoming`].
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
/// A simple multithreaded async echo server:
/// ```no_run
/// use std::io::Result;
/// use osiris::net::{TcpListener, TcpStream};
/// use osiris::buf::IoBuf;
/// use osiris::detach;
///
/// async fn handle_client(stream: TcpStream) -> Result<()> {
///     let buf = vec![0; 2048];
///     let (n, buf) = stream.read(buf).await;
///     let buf = buf.slice(..n?);
///     stream.write_all(buf).await.0?;
///     stream.close().await
/// }
///
/// #[osiris::main(scale = true)]
/// async fn main() -> Result<()> {
///     let listener = TcpListener::bind("127.0.0.1:8000").await?;    
///     loop {
///         let (stream, _) = listener.accept().await?;
///         detach(handle_client(stream));
///     }
///     Ok(())
/// }
/// ```

pub struct TcpListener {
    socket: Socket,
}

impl TcpListener {
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> Result<TcpListener> {
        try_until_success(addr, |addr| async move {
            let domain = Domain::from(addr);
            let socket = Socket::new(domain, Type::STREAM, Protocol::TCP).await?;
            socket.set_reuseport()?;
            socket.bind(&addr)?;
            socket.listen(128)?;
            Ok(TcpListener { socket })
        })
        .await
    }

    pub async fn accept(&self) -> Result<(TcpStream, SocketAddr)> {
        let (socket, addr) = self.socket.accept().await?;
        Ok((TcpStream { socket }, addr))
    }

    pub async fn close(self) -> Result<()> {
        self.socket.close().await
    }
}

#[test]
fn reuseport() {
    crate::block_on(async {
        let _listener1 = TcpListener::bind("127.0.0.1:8080").await.unwrap();
        let _listener2 = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    })
    .unwrap();
}
