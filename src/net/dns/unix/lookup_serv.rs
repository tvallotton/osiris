use std::io::Result;
use std::str::from_utf8;

use crate::net::socket::Protocol;
use crate::net::utils::{is_whitespace, lines, remove_comment};
#[derive(Debug, Clone)]
pub struct Service {
    port: u16,
    proto: Protocol,
}

async fn lookup_serv(
    services: &mut [Service],
    name: Option<&[u8]>,
    proto: Option<Protocol>,
) -> Result<usize> {
    let mut lines = lines("/etc/services", 8 * 1024).await?;
    let mut len = 0;
    while let Some(line) = lines.next().await? {
        let Some(service) = parse_line(name, proto, line) else {
            continue;
        };
        services[len] = service;
        len += 1;
        if len == services.len() {
            break;
        }
    }

    Ok(len)
}

fn parse_line(name: Option<&[u8]>, protocol: Option<Protocol>, line: &[u8]) -> Option<Service> {
    let line = remove_comment(line);
    let mut columns = line.split(is_whitespace).filter(|name| !name.is_empty());
    let serv = columns.next()?;
    let info = columns.next()?;
    let mut info = info.split(|c| *c == b'/');
    let port = info.next()?;
    let proto = info.next()?;
    let proto = match proto {
        b"udp" => Protocol::UDP,
        b"tcp" => Protocol::TCP,
        _ => return None,
    };

    let found_name = name == Some(serv);
    let mut found_alias = || columns.any(|alias| Some(alias) == name);

    if name.is_some() && !(found_name || found_alias()) {
        return None;
    }

    if protocol.is_some() && Some(proto) != protocol {
        return None;
    }

    let port = from_utf8(port).ok()?.parse().ok()?;
    Some(Service { port, proto })
}

#[test]
fn lookup_services() {
    let services = &mut vec![
        Service {
            port: 0,
            proto: Protocol::TCP,
        };
        1
    ];
    crate::block_on(async {
        // test alias
        lookup_serv(services, Some("postgresql".as_bytes()), None)
            .await
            .unwrap();
        assert_eq!(services[0].port, 5432);
        assert_eq!(services[0].proto, Protocol::TCP);
        // test alias
        let len = lookup_serv(services, Some("postgres".as_bytes()), None)
            .await
            .unwrap();
        assert_eq!(len, 1);
    })
    .unwrap();
}
