#![allow(clippy::transmute_ptr_to_ref)]
#![allow(unused_imports)]
use std::array::from_fn;
use std::future::{ready, Future, Ready};
use std::io::{Error, ErrorKind, Result};
use std::mem::{size_of, transmute};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::option;
use std::task::Poll;
/// A trait for objects which can be converted or resolved to one or more
/// [`SocketAddr`] values.
///
/// This trait is used for generic address resolution when constructing network
/// objects. By default it is implemented for the following types:
///
///  * [`SocketAddr`]: [`to_socket_addrs`] is the identity function.
///
///  * [`SocketAddrV4`], [`SocketAddrV6`], <code>([IpAddr], [u16])</code>,
///    <code>([Ipv4Addr], [u16])</code>, <code>([Ipv6Addr], [u16])</code>:
///    [`to_socket_addrs`] constructs a [`SocketAddr`] trivially.
///
///  * <code>(&[str], [u16])</code>: <code>&[str]</code> should be either a string representation
///    of an [`IpAddr`] address as expected by [`FromStr`] implementation or a host
///    name. [`u16`] is the port number.
///
///  * <code>&[str]</code>: the string should be either a string representation of a
///    [`SocketAddr`] as expected by its [`FromStr`] implementation or a string like
///    `<host_name>:<port>` pair where `<port>` is a [`u16`] value.
///
/// This trait allows constructing network objects like [`TcpStream`] or
/// [`UdpSocket`] easily with values of various types for the bind/connection
/// address. It is needed because sometimes one type is more appropriate than
/// the other: for simple uses a string like `"localhost:12345"` is much nicer
/// than manual construction of the corresponding [`SocketAddr`], but sometimes
/// [`SocketAddr`] value is *the* main source of the address, and converting it to
/// some other type (e.g., a string) just for it to be converted back to
/// [`SocketAddr`] in constructor methods is pointless.
///
/// Addresses returned by the operating system that are not IP addresses are
/// silently ignored.
///
/// [`FromStr`]: crate::str::FromStr "std::str::FromStr"
/// [`TcpStream`]: crate::net::TcpStream "net::TcpStream"
/// [`to_socket_addrs`]: ToSocketAddrs::to_socket_addrs
/// [`UdpSocket`]: crate::net::UdpSocket "net::UdpSocket"
///
/// # Examples
///
/// Creating a [`SocketAddr`] iterator that yields one item:
///
/// ```
/// use std::net::{ToSocketAddrs, SocketAddr};
///
/// let addr = SocketAddr::from(([127, 0, 0, 1], 443));
/// let mut addrs_iter = addr.to_socket_addrs().unwrap();
///
/// assert_eq!(Some(addr), addrs_iter.next());
/// assert!(addrs_iter.next().is_none());
/// ```
///
/// Creating a [`SocketAddr`] iterator from a hostname:
///
/// ```no_run
/// use std::net::{SocketAddr, ToSocketAddrs};
///
/// // assuming 'localhost' resolves to 127.0.0.1
/// let mut addrs_iter = "localhost:443".to_socket_addrs().unwrap();
/// assert_eq!(addrs_iter.next(), Some(SocketAddr::from(([127, 0, 0, 1], 443))));
/// assert!(addrs_iter.next().is_none());
///
/// // assuming 'foo' does not resolve
/// assert!("foo:443".to_socket_addrs().is_err());
/// ```
///
/// Creating a [`SocketAddr`] iterator that yields multiple items:
///
/// ```
/// use std::net::{SocketAddr, ToSocketAddrs};
///
/// let addr1 = SocketAddr::from(([0, 0, 0, 0], 80));
/// let addr2 = SocketAddr::from(([127, 0, 0, 1], 443));
/// let addrs = vec![addr1, addr2];
///
/// let mut addrs_iter = (&addrs[..]).to_socket_addrs().unwrap();
///
/// assert_eq!(Some(addr1), addrs_iter.next());
/// assert_eq!(Some(addr2), addrs_iter.next());
/// assert!(addrs_iter.next().is_none());
/// ```
///
/// Attempting to create a [`SocketAddr`] iterator from an improperly formatted
/// socket address `&str` (missing the port):
///
/// ```
/// use std::io;
/// use std::net::ToSocketAddrs;
///
/// let err = "127.0.0.1".to_socket_addrs().unwrap_err();
/// assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
/// ```
///
/// [`TcpStream::connect`] is an example of an function that utilizes
/// `ToSocketAddrs` as a trait bound on its parameter in order to accept
/// different types:
///
/// ```no_run
/// use std::net::{TcpStream, Ipv4Addr};
///
/// let stream = TcpStream::connect(("127.0.0.1", 443));
/// // or
/// let stream = TcpStream::connect("127.0.0.1:443");
/// // or
/// let stream = TcpStream::connect((Ipv4Addr::new(127, 0, 0, 1), 443));
/// ```
///
/// [`TcpStream::connect`]: crate::net::TcpStream::connect
pub trait ToSocketAddrs {
    /// Returned iterator over socket addresses which this type may correspond
    /// to.

    type Iter: Iterator<Item = SocketAddr>;

    /// Future to be returned from `to_socket_addrs`. This type will be removed when
    /// async fn is fully supported in traits
    type Output<'a>: Future<Output = Result<Self::Iter>> + 'a
    where
        Self: 'a;

    /// Converts this object to an iterator of resolved [`SocketAddr`]s.
    ///
    /// The returned iterator might not actually yield any values depending on the
    /// outcome of any resolution performed.
    ///

    fn to_socket_addrs(&self) -> Self::Output<'_>;
}

fn std_to_socket_addr<T: std::net::ToSocketAddrs>(addr: &T) -> Result<T::Iter> {
    addr.to_socket_addrs()
}

impl ToSocketAddrs for SocketAddr {
    type Iter = option::IntoIter<SocketAddr>;
    type Output<'a> = Ready<Result<Self::Iter>> where Self: 'a;
    fn to_socket_addrs(&self) -> Self::Output<'_> {
        ready(Ok(Some(*self).into_iter()))
    }
}

impl ToSocketAddrs for SocketAddrV4 {
    type Iter = option::IntoIter<SocketAddr>;
    type Output<'a> = Ready<Result<Self::Iter>> where Self: 'a;
    fn to_socket_addrs(&self) -> Self::Output<'_> {
        SocketAddr::V4(*self).to_socket_addrs()
    }
}

impl ToSocketAddrs for SocketAddrV6 {
    type Iter = option::IntoIter<SocketAddr>;
    type Output<'a> = Ready<Result<Self::Iter>> where Self: 'a;
    fn to_socket_addrs(&self) -> Self::Output<'_> {
        SocketAddr::V6(*self).to_socket_addrs()
    }
}

impl ToSocketAddrs for (IpAddr, u16) {
    type Iter = option::IntoIter<SocketAddr>;
    type Output<'a> = Ready<Result<Self::Iter>> where Self: 'a;
    fn to_socket_addrs(&self) -> Self::Output<'_> {
        let (ip, port) = *self;
        match ip {
            IpAddr::V4(ref a) => (*a, port).to_socket_addrs(),
            IpAddr::V6(ref a) => (*a, port).to_socket_addrs(),
        }
    }
}

impl ToSocketAddrs for (Ipv4Addr, u16) {
    type Iter = option::IntoIter<SocketAddr>;
    type Output<'a> = Ready<Result<Self::Iter>> where Self: 'a;
    fn to_socket_addrs(&self) -> Self::Output<'_> {
        let (ip, port) = *self;
        SocketAddrV4::new(ip, port).to_socket_addrs()
    }
}

impl ToSocketAddrs for (Ipv6Addr, u16) {
    type Iter = option::IntoIter<SocketAddr>;
    type Output<'a> = Ready<Result<Self::Iter>> where Self: 'a;
    fn to_socket_addrs(&self) -> Self::Output<'_> {
        let (ip, port) = *self;
        SocketAddrV6::new(ip, port, 0, 0).to_socket_addrs()
    }
}
