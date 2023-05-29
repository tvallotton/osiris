//! Implementation of `lookup` for Unix systems.
//!
//! This is largely based on the lookup system used in musl libc. Main differences:
//!
//! - Files are read asynchronously.
//! - We check for AAAA addresses after checking for A addresses.
//! - Instead of manually waiting for sockets to become readable, we use several sockets
//!   spawned on different tasks and polled using an executor.
//! - We use a more structured DNS protocol implementation instead of messy raw byte manipulation.
//! - The `memchr` crate is used to optimize certain operations.

use crate::fs::read_to_string;

use std::io::Result;
use std::net::IpAddr;

pub(super) async fn lookup(name: &str) -> Result<Option<IpAddr>> {
    // // We may be able to use the /etc/hosts resolver.
    // if let Some(addr) = from_hosts(name).await? {
    //     return Ok(addr);
    // }
    from_hosts(name).await
    // // Otherwise, we need to use the manual resolver.
    // let resolv = ResolvConf::load().await?;
    // dns_with_search(name, &resolv).await
}

/// Try parsing the name from the "hosts" file.
async fn from_hosts(name: &str) -> Result<Option<IpAddr>> {
    // TODO: do not read the file all at once.
    let hosts = read_to_string("/etc/hosts").await?;
    for line in hosts.lines() {
        let mut columns = line.split_ascii_whitespace();
        let Some(addr) = columns.next() else { continue };
        for hostname in columns {
            if name == hostname {
                return Ok(addr.parse().ok());
            }
        }
    }
    Ok(None)
}

/// Structural form of `resolv.conf`.
#[derive(Clone, Debug)]
struct ResolvConf {
    /// The list of name servers.
    name_servers: Vec<IpAddr>,

    /// Maximum number of segments in the domain name.
    ndots: u8,

    /// Maximum timeout in seconds.
    timeout: u8,

    /// Maximum number of retries.
    attempts: u8,

    /// The search domain to use.
    search: Option<String>,
}

impl Default for ResolvConf {
    fn default() -> Self {
        ResolvConf {
            name_servers: vec![],
            ndots: 1,
            timeout: 5,
            attempts: 2,
            search: None,
        }
    }
}

impl ResolvConf {
    async fn load() -> Self {
        let mut out = Self::default();
        out.try_load().await.ok();
        out
    }

    async fn try_load(&mut self) -> Result<()> {
        let conf = read_to_string("/etc/resolve.conf").await?;
        for line in conf.lines() {
            let mut columns = line.split_ascii_whitespace();
            let Some(key) = columns.next() else { continue };
            let Some(value) = columns.next() else { continue };
            if key == "nameserver" {
                let Ok(ip) =  value.parse() else { continue };
                self.name_servers.push(ip);
            }
        }
        Ok(())
    }
}

#[test]
fn lookup_from_host_test() {
    crate::block_on(async { dbg!(from_hosts("localhost").await) })
        .unwrap()
        .unwrap()
        .unwrap();
}

#[test]
fn resolve_conf_load_test() {
    crate::block_on(async { dbg!(ResolvConf::load().await) }).unwrap();
}
