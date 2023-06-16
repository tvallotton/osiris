use crate::net::utils::{is_whitespace, lines, remove_comment};
use resolv::ResolvConf;
use std::{io::Result, net::IpAddr, str::from_utf8};

mod lookup_serv;
mod resolv;
mod search;
pub async fn lookup(name: &str) -> Result<Vec<IpAddr>> {
    // // We may be able to use the /etc/hosts resolver.
    let addr = from_hosts(name).await?;
    if let Some(addr) = addr {
        return Ok(vec![addr]);
    }

    let resolv = ResolvConf::load();
    search::dns_search(name, &resolv).await
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

#[cfg(test)]
#[crate::test]
fn lookup_test() {
    crate::block_on(async {
        let ips = dbg!(lookup("www.wikipedia.com").await.unwrap());
        assert!(dbg!(ips).contains(&"208.80.154.232".parse().unwrap()))
    })
    .unwrap();
}

#[test]
fn lookup_non_existent_test() {
    crate::block_on(async {
        let ips = dbg!(lookup("www.non-existent-host.com").await.unwrap());
        assert!(dbg!(ips).is_empty());
    })
    .unwrap();
}
