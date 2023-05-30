use std::{io::Result, str::from_utf8};

use socket2::Protocol;

use crate::net::utils::{is_whitespace, lines, remove_comment};

pub struct Service {
    port: u16,
    proto: Protocol,
}
// TODO: do not read the whole file at once
async fn lookup_serv(
    services: &mut [Service],
    name: Option<&[u8]>,
    proto: Option<Protocol>,
) -> Result<usize> {
    let mut lines = lines("/etc/services", 4 * 1024).await?;
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
    let mut columns = line.split(is_whitespace);
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
    if name != Some(serv) && name.is_some() {
        return None;
    }

    if Some(proto) != protocol && protocol.is_some() {
        return None;
    }

    let port = from_utf8(port).ok()?.parse().ok()?;
    Some(Service { port, proto })
}
