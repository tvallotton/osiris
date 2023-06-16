use std::future::{ready, Ready};
use std::io::{Error, ErrorKind, Result};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};
use std::pin::Pin;
use std::vec;
use std::{future::Future, net::SocketAddr};

use crate::net::dns;

use super::utils::invalid_input;

pub trait ToSocketAddrs {
    /// Returned iterator over socket addresses which this type may correspond
    /// to.

    type Iter: Iterator<Item = SocketAddr>;

    type Fut: Future<Output = Result<Self::Iter>>;

    /// Converts this object to an iterator of resolved [`SocketAddr`]s.
    ///
    /// The returned iterator might not actually yield any values depending on the
    /// outcome of any resolution performed.
    ///
    /// Note that this function may block the current thread while resolution is
    /// performed.
    fn to_socket_addrs(self) -> Self::Fut;
}

/// Uses std's impl of ToSocketAddr
macro_rules! use_std {
    ($($ty:ty),*) => {$(
        impl<'a> ToSocketAddrs for &'a $ty {
            type Iter = <Self as std::net::ToSocketAddrs>::Iter;
            type Fut = Ready<Result<Self::Iter>>;
            fn to_socket_addrs(self) -> Self::Fut {
                ready(std::net::ToSocketAddrs::to_socket_addrs(&self))
            }
        }
    )*};
}

use_std! {
    SocketAddr,
    SocketAddrV4,
    SocketAddrV6,
    (IpAddr, u16),
    (Ipv4Addr, u16),
    (Ipv6Addr, u16)
}

impl<'a> ToSocketAddrs for &'a (&str, u16) {
    type Iter = vec::IntoIter<SocketAddr>;
    type Fut = Pin<Box<dyn Future<Output = Result<Self::Iter>> + 'a>>;
    fn to_socket_addrs(self) -> Self::Fut {
        Box::pin(async {
            let (host, port) = *self;
            let res = dns::lookup(host)
                .await?
                .map(|ip_addr| SocketAddr::from((ip_addr, port)));
            Ok(res.collect::<Vec<_>>().into_iter())
        })
    }
}

impl<'a> ToSocketAddrs for &'a (String, u16) {
    type Iter = vec::IntoIter<SocketAddr>;
    type Fut = Pin<Box<dyn Future<Output = Result<Self::Iter>> + 'a>>;
    fn to_socket_addrs(self) -> Self::Fut {
        // (&*self.0, self.1).to_socket_addrs()
        todo!()
    }
}

// accepts strings like 'localhost:12345'
impl<'a> ToSocketAddrs for &'a str {
    type Iter = vec::IntoIter<SocketAddr>;
    type Fut = Pin<Box<dyn Future<Output = Result<Self::Iter>> + 'a>>;
    fn to_socket_addrs(self) -> Self::Fut {
        Box::pin(async {
            let (host, port) = self.split_once(':').ok_or_else(invalid_input)?;
            let port: u16 = port
                .parse()
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "invalid port value"))?;
            let out = dns::lookup(host)
                .await?
                .map(|addr| SocketAddr::from((addr, port)))
                .collect::<Vec<_>>()
                .into_iter();
            Ok(out)
        })
    }
}

impl<'a> ToSocketAddrs for &'a [SocketAddr] {
    type Iter = std::iter::Cloned<std::slice::Iter<'a, SocketAddr>>;
    type Fut = Ready<Result<Self::Iter>>;
    fn to_socket_addrs(self) -> Self::Fut {
        ready(Ok(self.iter().cloned()))
    }
}

impl<'a> ToSocketAddrs for &'a String {
    type Iter = vec::IntoIter<SocketAddr>;
    type Fut = Pin<Box<dyn Future<Output = Result<Self::Iter>> + 'a>>;
    fn to_socket_addrs(self) -> Self::Fut {
        (&**self).to_socket_addrs()
    }
}

impl<T: ToSocketAddrs + ?Sized> ToSocketAddrs for &T {
    type Iter = T::Iter;
    type Fut = T::Fut;
    fn to_socket_addrs(self) -> Self::Fut {
        // (*self).to_socket_addrs()
        todo!()
    }
}

pub(crate) async fn try_until_success<A, T, F, Ft>(addr: A, mut f: F) -> Result<T>
where
    for<'a> &'a A: ToSocketAddrs,
    F: FnMut(SocketAddr) -> Ft,
    Ft: Future<Output = Result<T>>,
{
    let mut error = None;
    for addr in addr.to_socket_addrs().await? {
        let result = f(addr).await;
        let Err(err) = result else {
                return result;
            };
        error = Some(err);
    }
    Err(error.unwrap_or_else(invalid_input))
}
