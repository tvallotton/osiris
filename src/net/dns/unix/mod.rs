use crate::net::utils::{is_whitespace, lines, remove_comment};
use resolv::ResolvConf;
use std::{io::Result, net::IpAddr, str::from_utf8};

mod lookup_serv;
mod resolv;

pub async fn lookup(name: &str) -> Result<Option<IpAddr>> {
    // // We may be able to use the /etc/hosts resolver.
    let addr = from_hosts(name).await?;
    if addr.is_some() {
        return Ok(addr);
    }

    let _resolv = ResolvConf::load();
    todo!()
}

/// Try parsing the name from the "hosts" file.
async fn from_hosts(name: &str) -> Result<Option<IpAddr>> {
    let mut lines = lines("/etc/hosts", 1024).await?;

    while let Some(line) = lines.next().await? {
        let line = remove_comment(line);

        let mut columns = line.split(is_whitespace);

        let Some(addr) = columns.next() else { continue };
        for hostname in columns {
            if name.as_bytes() == hostname {
                let Ok(addr) = from_utf8(addr) else { continue };
                return Ok(addr.parse().ok());
            }
        }
    }
    Ok(None)
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
    crate::block_on(async { dbg!(ResolvConf::load()) }).unwrap();
}
