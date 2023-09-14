#![allow(non_camel_case_types)]
/// We need to copy this because libc::statx
/// is not available in musl
#[derive(Clone, Copy)]
#[repr(C)]
pub struct statx {
    pub stx_mask: u32,
    pub stx_blksize: u32,
    pub stx_attributes: u64,
    pub stx_nlink: u32,
    pub stx_uid: u32,
    pub stx_gid: u32,
    pub stx_mode: u16,
    pub __statx_pad1: [u16; 1],
    pub stx_ino: u64,
    pub stx_size: u64,
    pub stx_blocks: u64,
    pub stx_attributes_mask: u64,
    pub stx_atime: statx_timestamp,
    pub stx_btime: statx_timestamp,
    pub stx_ctime: statx_timestamp,
    pub stx_mtime: statx_timestamp,
    pub stx_rdev_major: u32,
    pub stx_rdev_minor: u32,
    pub stx_dev_major: u32,
    pub stx_dev_minor: u32,
    pub __statx_pad2: [u64; 14],
}
#[derive(Clone, Copy)]
#[repr(C)]
pub struct statx_timestamp {
    pub tv_sec: i64,
    pub tv_nsec: u32,
    pub __statx_timestamp_pad1: [i32; 1],
}
pub const STATX_ALL: u32 = 0x0fff;

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
