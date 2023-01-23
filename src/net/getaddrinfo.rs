#![allow(clippy::char_lit_as_u8)]
#![allow(warnings)]
use std::ffi::{c_char, CStr};
use std::io;

use std::mem::MaybeUninit;
use std::ptr::{null, null_mut};

use libc::{
    addrinfo, c_uchar, in6_addr, in_addr, sockaddr_in, sockaddr_in6, AF_INET, AF_INET6, AF_UNSPEC,
    AI_ALL, AI_NUMERICHOST, AI_NUMERICSERV, AI_PASSIVE, AI_V4MAPPED, EAI_BADFLAGS, EAI_FAMILY,
    EAI_NONAME, IPPROTO_UDP, PF_UNSPEC, SOCK_CLOEXEC, SOCK_DGRAM,
};

#[repr(C)]
struct gaih_service {
    name: *const libc::c_char,
    num: libc::c_int,
}

const in6ai_deprecated: u8 = 1;
const in6ai_homeaddress: u8 = 2;

#[repr(C)]
struct in6addrinfo {
    flags: u8,
    prefixlen: u8,
    __pad: u16,
    index: u32,
    addr: [u32; 4],
}

const AI_IDN: i32 = 0x0040; //IDN encode input (assuming it is encoded
const AI_V4MAPPED_CFG: i32 = 0x00000200; // accept IPv4-mapped if kernel supports
const AI_ADDRCONFIG: i32 = 0x00000400; // only if any address is assigned
const AI_DEFAULT: i32 = AI_V4MAPPED_CFG | AI_ADDRCONFIG;
const AI_CANONIDN: i32 = 0x0080; // Translate canonical name from IDN format.
const DEPRECATED_AI_IDN: i32 = 0x300; // Former AI_IDN_ALLOW_UNASSIGNED and AI_IDN_USE_STD3_ASCII_RULES flags, now ignored.
const AI_CANONNAME: i32 = 0x00000002; // only if any address is assigned
const default_hints: libc::addrinfo = libc::addrinfo {
    ai_flags: AI_DEFAULT,
    ai_family: libc::PF_UNSPEC,
    ai_socktype: 0,
    ai_protocol: 0,
    ai_addrlen: 0,
    ai_addr: null_mut(),
    ai_canonname: null_mut(),
    ai_next: null_mut(),
};
#[inline]
fn cmp(s1: Option<&CStr>, s2: &[u8]) -> bool {
    s1.map(CStr::to_bytes) == Some(s2)
}

fn is_wildcard(name: Option<&CStr>) -> bool {
    cmp(name, b"*\0")
}

pub fn error<T>(code: i32) -> io::Result<T> {
    Err(std::io::Error::from_raw_os_error(code))
}

pub fn all(flags: i32, enabled: i32) -> bool {
    (flags & enabled) != 0
}

/* The limit of 48 results is a non-sharp bound on the number of addresses
 * that can fit in one 512-byte DNS packet full of v4 results and a second
 * packet full of v6 results. Due to headers, the actual limit is lower. */
const MAXADDRS: usize = 48;
const MAXSERVS: usize = 2;

#[repr(C)]
struct service {
    port: u64,
    proto: c_uchar,
    socktype: c_uchar,
}
#[repr(C)]
struct address {
    family: i32,
    scopeid: u32,
    addr: [u8; 16],
    sortkey: i32,
}

const fn bad_flags(hint: &libc::addrinfo) -> bool {
    const MASK: i32 = AI_PASSIVE
        | AI_CANONNAME
        | AI_NUMERICHOST
        | AI_V4MAPPED
        | AI_ALL
        | AI_ADDRCONFIG
        | AI_NUMERICSERV;

    hint.ai_flags & MASK != hint.ai_flags
}

const fn size_of<T>(_: &T) -> u32 {
    std::mem::size_of::<T>() as _
}

async fn socket(domain: i32, type_: i32, protocol: i32) -> i32 {
    todo!()
}
async fn connect(s: i32, addr: *const (), size: u32) -> i32 {
    todo!()
}
async fn close(s: i32) {
    todo!()
}
async fn getaddrinfo(
    host: Option<&str>,
    serv: Option<&str>,
    hint: Option<&addrinfo>,
    res: &mut addrinfo,
) -> Result<addrinfo, io::Error> {
    let ports: [service; MAXSERVS];
    let addrs: [address; MAXADDRS];

    let canon = MaybeUninit::<[c_char; 256]>::uninit();
    let outcanon: &mut c_char;

    let nservs = 0;
    let naddrs = 0;
    let nais = 0;
    let canon_len = 0;
    let i = 0;
    let j = 0;
    let k = 0;

    let mut family = AF_UNSPEC;
    let mut flags = 0;
    let mut proto = 0;
    let mut socktype = 0;

    if host.is_none() && serv.is_none() {
        return error(EAI_NONAME);
    }

    if let Some(hint) = hint {
        family = hint.ai_family;
        flags = hint.ai_flags;
        proto = hint.ai_protocol;
        socktype = hint.ai_socktype;

        if bad_flags(&hint) {
            return error(EAI_BADFLAGS);
        }
        let (AF_INET | AF_INET6| AF_UNSPEC) = family else {
            return error(EAI_FAMILY)
        };
    }

    if (flags & AI_ADDRCONFIG) != 0 {
        // Define the "an address is configured" condition for address
        // families via ability to create a socket for the family plus
        // routability of the loopback address for the family.
        const lo4: sockaddr_in = sockaddr_in {
            sin_family: AF_INET as _,
            sin_port: 65535,
            sin_zero: [0; 8],
            sin_addr: in_addr {
                s_addr: u32::to_be(0x7f000001),
            },
        };

        const lo6: sockaddr_in6 = sockaddr_in6 {
            sin6_family: AF_INET6 as _,
            sin6_port: 65535,
            sin6_addr: in6_addr { s6_addr: [0; 16] },
            sin6_flowinfo: 0,
            sin6_scope_id: 0,
        };
        const tf: [i32; 2] = [AF_INET, AF_INET6];
        const ta: [*const (); 2] = [&lo4 as *const _ as _, &lo6 as *const _ as _];
        const tl: [u32; 2] = [size_of(&lo4), size_of(&lo6)];

        for i in 0..2 {
            if family == tf[1 - i] {
                continue;
            }
            let s = socket(tf[i], SOCK_CLOEXEC | SOCK_DGRAM, IPPROTO_UDP).await;

            if s > 0 {
                let r = connect(s, ta[i], tl[i]).await;
                let errorno = io::Error::last_os_error();
                close(s).await;
                if (r == 0) {
                    continue;
                }
                // errno = saved_errno;
            }

            // 		switch (errno) {
            // 		case EADDRNOTAVAIL:
            // 		case EAFNOSUPPORT:
            // 		case EHOSTUNREACH:
            // 		case ENETDOWN:
            // 		case ENETUNREACH:
            // 			break;
            // 		default:
            // 			return EAI_SYSTEM;
            // 		}
            // 		if (family == tf[i]) no_family = 1;
            // 		family = tf[1-i];
        }
    }

    // nservs = __lookup_serv(ports, serv, proto, socktype, flags);
    // if (nservs < 0) return nservs;

    // naddrs = __lookup_name(addrs, canon, host, family, flags);
    // if (naddrs < 0) return naddrs;

    // if (no_family) return EAI_NODATA;

    // nais = nservs * naddrs;
    // canon_len = strlen(canon);
    // out = calloc(1, nais * sizeof(*out) + canon_len + 1);
    // if (!out) return EAI_MEMORY;

    // if (canon_len) {
    // 	outcanon = (void *)&out[nais];
    // 	memcpy(outcanon, canon, canon_len+1);
    // } else {
    // 	outcanon = 0;
    // }

    // for (k=i=0; i<naddrs; i++) for (j=0; j<nservs; j++, k++) {
    // 	out[k].slot = k;
    // 	out[k].ai = (struct addrinfo){
    // 		.ai_family = addrs[i].family,
    // 		.ai_socktype = ports[j].socktype,
    // 		.ai_protocol = ports[j].proto,
    // 		.ai_addrlen = addrs[i].family == AF_INET
    // 			? sizeof(struct sockaddr_in)
    // 			: sizeof(struct sockaddr_in6),
    // 		.ai_addr = (void *)&out[k].sa,
    // 		.ai_canonname = outcanon };
    // 	if (k) out[k-1].ai.ai_next = &out[k].ai;
    // 	switch (addrs[i].family) {
    // 	case AF_INET:
    // 		out[k].sa.sin.sin_family = AF_INET;
    // 		out[k].sa.sin.sin_port = htons(ports[j].port);
    // 		memcpy(&out[k].sa.sin.sin_addr, &addrs[i].addr, 4);
    // 		break;
    // 	case AF_INET6:
    // 		out[k].sa.sin6.sin6_family = AF_INET6;
    // 		out[k].sa.sin6.sin6_port = htons(ports[j].port);
    // 		out[k].sa.sin6.sin6_scope_id = addrs[i].scopeid;
    // 		memcpy(&out[k].sa.sin6.sin6_addr, &addrs[i].addr, 16);
    // 		break;
    // 	}
    // }
    // out[0].ref = nais;
    // *res = &out->ai;
    // return 0;
    todo!()
}

// async fn getaddrinfo2(
//     mut name: Option<&CStr>,
//     mut service: Option<&CStr>,
//     hints: Option<&libc::addrinfo>,
// ) -> io::Result<()> {
//     let mut gaih_service = gaih_service {
//         name: null(),
//         num:0,
//     };

//     if is_wildcard(name) {
//         name = None;
//     }
//     if is_wildcard(service) {
//         service = None;
//     }

//     if name.is_none() && service.is_none() {
//         return error(EAI_NONAME);
//     }

//     let hints = hints.unwrap_or(&default_hints);

//     if bad_flags(&hints) {
//         return error(EAI_BADFLAGS);
//     }

//     if (hints.ai_flags & AI_CANONNAME) != 0 && name.is_none() {
//         return error(EAI_BADFLAGS);
//     }

//     let ai_family = hints.ai_family;

//     if ai_family != AF_UNSPEC && ai_family != AF_INET && ai_family != AF_INET6 {
//         return error(EAI_FAMILY);
//     }

//     let in6ai: Option<in6addrinfo> = None;
//     let mut in6ailen: usize = 0;
//     let mut seen_ipv4 = false;
//     let mut seen_ipv6 = false;
//     let mut check_pf_called = false;

//     if hints.ai_flags & AI_ADDRCONFIG != 0 {
//         // We might need information about what interfaces are available.
//         // Also determine whether we have IPv4 or IPv6 interfaces or both. We
//         // cannot cache the results since new interfaces could be added at
//         // any time.
//         __check_pf(&mut seen_ipv4, &mut seen_ipv6, &mut in6ai, &mut in6ailen);
//         check_pf_called = true;

//         // Now make a decision on what we return, if anything.
//         if ai_family == PF_UNSPEC && (seen_ipv4 || seen_ipv6) {
//             // If we haven't seen both IPv4 and IPv6 interfaces we can
//             // narrow down the search.
//             if seen_ipv4 != seen_ipv6 {
//                 local_hints = *hints;
//                 local_hints.ai_family = if seen_ipv4 { PF_INET } else { PF_INET6 };
//                 hints = &local_hints;
//             }
//         } else if (ai_family == PF_INET && !seen_ipv4) || (ai_family == PF_INET6 && !seen_ipv6) {
//             // We cannot possibly return a valid answer.
//             __free_in6ai(in6ai);
//             return error(EAI_NONAME);
//         }
//     }

//     if cmp(service, b"\0") {

//       gaih_service.name = service.unwrap();
//       gaih_service.num = strtoul (gaih_service.name, &c, 10);
//       if (*c != '\0')
// 	{
// 	  if (hints->ai_flags & AI_NUMERICSERV)
// 	    {
// 	      __free_in6ai (in6ai);
// 	      return EAI_NONAME;
// 	    }

// 	  gaih_service.num = -1;
// 	}

//       pservice = &gaih_service;
//     }

//     todo!()
// }
