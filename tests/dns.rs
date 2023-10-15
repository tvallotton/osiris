use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

use osiris::net::ToSocketAddrs;

pub fn sa4(a: Ipv4Addr, p: u16) -> SocketAddr {
    SocketAddr::V4(SocketAddrV4::new(a, p))
}

pub fn sa6(a: Ipv6Addr, p: u16) -> SocketAddr {
    SocketAddr::V6(SocketAddrV6::new(a, p, 0, 0))
}

pub async fn tsa<A: ToSocketAddrs>(a: A) -> Result<Vec<SocketAddr>, String> {
    match a.to_socket_addrs().await {
        Ok(a) => Ok(a.collect()),
        Err(e) => Err(e.to_string()),
    }
}

#[osiris::test]
async fn to_socket_addr_ipaddr_u16() {
    let a = Ipv4Addr::new(77, 88, 21, 11);
    let p = 12345;
    let e = SocketAddr::V4(SocketAddrV4::new(a, p));
    assert_eq!(Ok(vec![e]), tsa((a, p)).await);
}

#[osiris::test]
async fn to_socket_addr_str_u16() {
    let a = sa4(Ipv4Addr::new(77, 88, 21, 11), 24352);
    assert_eq!(Ok(vec![a]), tsa(("77.88.21.11", 24352)).await);

    let a = sa6(Ipv6Addr::new(0x2a02, 0x6b8, 0, 1, 0, 0, 0, 1), 53);
    assert_eq!(Ok(vec![a]), tsa(("2a02:6b8:0:1::1", 53)).await);

    let a = sa4(Ipv4Addr::new(127, 0, 0, 1), 23924);
    #[cfg(not(target_env = "sgx"))]
    assert!(tsa(("localhost", 23924)).await.unwrap().contains(&a));
    #[cfg(target_env = "sgx")]
    let _ = a;
}

#[osiris::test]
async fn to_socket_addr_str() {
    let a = sa4(Ipv4Addr::new(77, 88, 21, 11), 24352);
    assert_eq!(Ok(vec![a]), tsa("77.88.21.11:24352").await);

    let a = sa6(Ipv6Addr::new(0x2a02, 0x6b8, 0, 1, 0, 0, 0, 1), 53);
    assert_eq!(Ok(vec![a]), tsa("[2a02:6b8:0:1::1]:53").await);

    let a = sa4(Ipv4Addr::new(127, 0, 0, 1), 23924);
    #[cfg(not(target_env = "sgx"))]
    assert!(tsa("localhost:23924").await.unwrap().contains(&a));
    #[cfg(target_env = "sgx")]
    let _ = a;
}

#[osiris::test]
async fn to_socket_addr_string() {
    let a = sa4(Ipv4Addr::new(77, 88, 21, 11), 24352);
    assert_eq!(
        Ok(vec![a]),
        tsa(&*format!("{}:{}", "77.88.21.11", "24352")).await
    );
    assert_eq!(
        Ok(vec![a]),
        tsa(&format!("{}:{}", "77.88.21.11", "24352")).await
    );

    // s has been moved into the tsa call
}

#[osiris::test]
async fn bind_udp_socket_bad() {
    // rust-lang/rust#53957: This is a regression test for a parsing problem
    // discovered as part of issue rust-lang/rust#23076, where we were
    // incorrectly parsing invalid input and then that would result in a
    // successful `UdpSocket` binding when we would expect failure.
    //
    // At one time, this test was written as a call to `tsa` with
    // INPUT_23076. However, that structure yields an unreliable test,
    // because it ends up passing junk input to the DNS server, and some DNS
    // servers will respond with `Ok` to such input, with the ip address of
    // the DNS server itself.
    //
    // This form of the test is more robust: even when the DNS server
    // returns its own address, it is still an error to bind a UDP socket to
    // a non-local address, and so we still get an error here in that case.

    const INPUT_23076: &str = "1200::AB00:1234::2552:7777:1313:34300";

    assert!(osiris::net::UdpSocket::bind(INPUT_23076).await.is_err())
}
