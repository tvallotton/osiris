#![allow(non_camel_case_types)]

pub use stat::{statx, statx_timestamp, STATX_BASIC_STATS};

pub(crate) mod buf;
pub(crate) mod futures;
pub(crate) mod stat;

#[repr(C)]
pub struct epoll_event {
    pub events: u32,
    pub u64: u64,
}

macro_rules! syscall {
    ($name: ident, $($args:expr),* $(,)?) => {{
        let res = unsafe {
            libc::$name($($args),*)
        };
        if res < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(res)
        }

    }};
}

#[allow(warnings)]
pub(crate) use syscall;
