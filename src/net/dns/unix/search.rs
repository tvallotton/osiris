/// This is ported from the async-dns crate, which itself is a port of musl
use std::{
    io::{Error, ErrorKind, Result},
    net::{IpAddr, SocketAddr},
    rc::Rc,
    time::Duration,
};

use dns_protocol::{Flags, Message, Question, ResourceRecord, ResourceType};

use crate::{buf::IoBuf, net::udp::UdpSocket, spawn, task::yield_now, time::timeout};

use super::resolv::ResolvConf;

/// Preform a DNS lookup, considering the search variable.
pub async fn dns_search(mut name: &str, resolv: &ResolvConf) -> Result<Vec<IpAddr>> {
    // See if we should just use global scope.
    let num_dots = memchr::Memchr::new(b'.', name.as_bytes()).count();
    let global_scope = num_dots >= resolv.ndots as usize || name.ends_with('.');

    // Remove the dots from the end of `name`, if needed.
    if name.ends_with('.') {
        name = &name[..name.len() - 1];

        // Raise an error if name still ends with a dot.
        if name.ends_with('.') {
            return Err(Error::new(ErrorKind::InvalidInput, "name ends with a dot"));
        }
    }

    if global_scope {
        if let Some(search) = resolv.search.as_ref() {
            // Try the name with the search domains.
            let mut buffer = String::from(name);
            buffer.push('.');
            let name_end = buffer.len();

            // Try the name with the search domains.
            for domain in search.split_whitespace() {
                buffer.truncate(name_end);
                buffer.push_str(domain);

                if let Ok(addrs) = dns_lookup(&buffer, resolv).await {
                    if !addrs.is_empty() {
                        return Ok(addrs);
                    }
                }
            }
        }
    }

    // Preform a DNS search on just the name.
    dns_lookup(name, resolv).await
}

/// Preform a manual lookup for the name.
async fn dns_lookup(name: &str, resolv: &ResolvConf) -> Result<Vec<IpAddr>> {
    match resolv.name_servers.len() {
        0 => {
            // No nameservers, so we can't do anything.
            Ok(vec![])
        }
        1 => {
            // Just poll the one nameserver.
            let addr = resolv.name_servers[0];
            query_name_and_nameserver(name, addr, resolv).await
        }
        _ => {
            // Use an executor to poll futures in parallel.

            let name = Rc::from(name.to_string().into_boxed_str());
            let resolv = Rc::new(resolv.clone());

            let mut handles = Vec::with_capacity(resolv.name_servers.len());
            for ns in resolv.name_servers.iter().copied() {
                let name = Rc::clone(&name);
                let resolv = resolv.clone();
                let handle =
                    spawn(async move { query_name_and_nameserver(&name, ns, &resolv).await });
                handles.push(handle);
            }
            let mut result = Vec::with_capacity(resolv.name_servers.len());
            for handle in handles {
                result.append(&mut handle.await?);
            }

            Ok(result)
        }
    }
}

/// Poll for the name on the given nameserver.
async fn query_name_and_nameserver(
    name: &str,
    nameserver: IpAddr,
    resolv: &ResolvConf,
) -> Result<Vec<IpAddr>> {
    // Try to poll for an IPv4 address first.
    let mut addrs =
        query_question_and_nameserver(Question::new(name, ResourceType::A, 1), nameserver, resolv)
            .await?;

    // If we didn't get any addresses, try an IPv6 address.
    if addrs.is_empty() {
        addrs = query_question_and_nameserver(
            Question::new(name, ResourceType::AAAA, 1),
            nameserver,
            resolv,
        )
        .await?;
    }

    Ok(addrs)
}

/// Poll for a DNS response on the given nameserver.
async fn query_question_and_nameserver(
    question: Question<'_>,
    nameserver: IpAddr,
    resolv: &ResolvConf,
) -> Result<Vec<IpAddr>> {
    // Create the DNS query.
    // I'd like to use two questions at once, but at least the DNS system I use just drops the packet.
    let id = fastrand::u16(..);
    let mut questions = [question];
    let message = Message::new(
        id,
        Flags::standard_query(),
        &mut questions,
        &mut [],
        &mut [],
        &mut [],
    );

    // Serialize it to a buffer.

    let needed = message.space_needed();
    let mut buf = vec![0; needed];

    let len = message
        .write(&mut buf)
        .map_err(|err| Error::new(ErrorKind::Other, err))?;
    let buf = Rc::new(buf.slice(0..len));

    // The query may be too large, so we need to use TCP.
    if len <= 512 {
        if let Some(addrs) = question_with_udp(id, buf.clone(), nameserver, resolv).await? {
            return Ok(addrs);
        }
    }

    // We were unable to complete the query over UDP, use TCP instead.
    question_with_tcp(id, buf, nameserver).await
}

/// Query a nameserver for the given question, using the UDP protocol.
///
/// Returns `None` if the UDP query failed and TCP should be used instead.
async fn question_with_udp(
    id: u16,
    query: impl IoBuf + Clone,
    nameserver: IpAddr,
    resolv: &ResolvConf,
) -> Result<Option<Vec<IpAddr>>> {
    const RECORD_BUFSIZE: usize = 16;

    /// The result of waiting for a packet on a fixed timeout.
    enum WaitResult {
        /// The packet was received.
        Packet { len: usize },
        /// The timeout expired.
        TimedOut,
    }

    let mut addrs = vec![];

    // Write the query to the nameserver address.
    let socket = UdpSocket::bind(("0.0.0.0", 0)).await?;
    let foreign_addr = SocketAddr::new(nameserver, 53);

    // UDP queries are limited to 512 bytes.
    let mut buf = vec![0; 512];

    for _ in 0..resolv.attempts {
        // Wait for `timeout` seconds for a response.
        socket.send_to(query.clone(), foreign_addr).await.0?;

        let duration = Duration::from_secs(resolv.timeout.into());
        let result = timeout(socket.recv(buf), duration).await;

        // Get the length of the packet we're reading.
        let len = match result {
            Ok((Ok(len), buf_)) => {
                buf = buf_;
                len
            }
            Ok((Err(_), buf_)) => {
                buf = buf_;
                yield_now().await;
                continue;
            }
            Err(_) => {
                buf = vec![0; 512];
                yield_now().await;
                continue;
            }
        };

        // Buffers for DNS results.
        let mut q_buf = [Question::default(); 1];
        let mut answers = [ResourceRecord::default(); RECORD_BUFSIZE];
        let mut authority = [ResourceRecord::default(); RECORD_BUFSIZE];
        let mut additional = [ResourceRecord::default(); RECORD_BUFSIZE];

        // Parse the packet.
        let message = Message::read(
            &buf[..len],
            &mut q_buf,
            &mut answers,
            &mut authority,
            &mut additional,
        )
        .map_err(|err| Error::new(ErrorKind::Other, err))?;

        // Check the ID.
        if message.id() != id {
            // Try again.
            yield_now().await;
            continue;
        }

        // If the reply was truncated, it's too large for UDP.
        if message.flags().truncated() {
            return Ok(None);
        }

        // Parse the resulting answer.
        parse_answers(&message, &mut addrs);

        // We got a response, so we're done.
        return Ok(Some(addrs));
    }

    // We did not receive a response.
    Ok(None)
}

/// Query a nameserver for the given question, using the TCP protocol.
#[cold]
async fn question_with_tcp(
    _id: u16,
    query: impl IoBuf,
    _nameserver: IpAddr,
) -> Result<Vec<IpAddr>> {
    const RECORD_BUFSIZE: usize = 16;

    if query.bytes_init() > u16::MAX as usize {
        return Err(Error::new(ErrorKind::Other, "query too large for TCP"));
    }
    todo!()
    // // Open the socket to the server.
    // let mut socket = Async::<TcpStream>::connect((nameserver, 53)).await?;

    // // Write the length of the query.
    // let len_bytes = (query.len() as u16).to_be_bytes();
    // socket.write_all(&len_bytes).await?;

    // // Write the query.
    // socket.write_all(query).await?;

    // // Read the length of the response.
    // let mut len_bytes = [0; 2];
    // socket.read_exact(&mut len_bytes).await?;
    // let len = u16::from_be_bytes(len_bytes) as usize;

    // // Read the response.
    // let mut stack_buffer = [0; 1024];
    // let mut heap_buffer;
    // let buf = if len > stack_buffer.len() {
    //     // Initialize the heap buffer and return a pointer to it.
    //     heap_buffer = vec![0; len];
    //     heap_buffer.as_mut_slice()
    // } else {
    //     &mut stack_buffer
    // };

    // socket.read_exact(buf).await?;

    // // Parse the response.
    // let mut q_buf = [Question::default(); 1];
    // let mut answers = [ResourceRecord::default(); RECORD_BUFSIZE];
    // let mut authority = [ResourceRecord::default(); RECORD_BUFSIZE];
    // let mut additional = [ResourceRecord::default(); RECORD_BUFSIZE];

    // let message = Message::read(
    //     &buf[..len],
    //     &mut q_buf,
    //     &mut answers,
    //     &mut authority,
    //     &mut additional,
    // )
    // .map_err(|err| Error::new(ErrorKind::Other, err))?;

    // if message.id() != id {
    //     return Err(Error::new(ErrorKind::Other, "invalid ID in response"));
    // }

    // // Parse the answers as address info.
    // let mut addrs = vec![];
    // parse_answers(&message, &mut addrs);
    // Ok(addrs)
}

/// Append address information to the vector, given the DNS response.
fn parse_answers(response: &Message<'_, '_>, addrs: &mut Vec<IpAddr>) {
    addrs.extend(response.answers().iter().filter_map(|answer| {
        let data = answer.data();

        // Parse the data as an IP address.
        match data.len() {
            4 => {
                let data: [u8; 4] = data.try_into().unwrap();
                Some(IpAddr::V4(data.into()))
            }
            16 => {
                let data: [u8; 16] = data.try_into().unwrap();
                Some(IpAddr::V6(data.into()))
            }
            _ => None,
        }
    }));
}
