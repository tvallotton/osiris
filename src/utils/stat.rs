pub const STATX_BASIC_STATS: u32 = 0x07ff;

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
    __statx_pad1: [u16; 1],
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
    __statx_pad2: [u64; 14],
}
#[derive(Clone, Copy)]
#[repr(C)]

pub struct statx_timestamp {
    pub tv_sec: i64,
    pub tv_nsec: u32,
    pub __statx_timestamp_pad1: [i32; 1],
}

impl statx {
    pub fn from_stat(stat: libc::stat) -> statx {
        unsafe {
            statx {
                stx_mask: STATX_BASIC_STATS,
                stx_blksize: stat.st_blksize as _,
                stx_attributes: 0,
                stx_nlink: stat.st_nlink as _,
                stx_uid: stat.st_uid,
                stx_gid: stat.st_gid,
                stx_mode: stat.st_mode as _,
                __statx_pad1: [0],
                stx_ino: stat.st_ino,
                stx_size: stat.st_size as _,
                stx_blocks: stat.st_blocks as _,
                stx_attributes_mask: 0,
                stx_atime: statx_timestamp::new(stat.st_atime, stat.st_atime_nsec),
                stx_btime: statx_timestamp::new(0, 0),
                stx_ctime: statx_timestamp::new(stat.st_ctime, stat.st_ctime_nsec),
                stx_mtime: statx_timestamp::new(stat.st_mtime, stat.st_mtime_nsec),
                stx_rdev_major: libc::major(stat.st_rdev) as _,
                stx_rdev_minor: libc::minor(stat.st_rdev) as _,
                stx_dev_major: libc::major(stat.st_dev) as _,
                stx_dev_minor: libc::minor(stat.st_dev) as _,
                __statx_pad2: [0; 14],
            }
        }
    }
}

impl statx_timestamp {
    fn new(sec: i64, nsec: i64) -> Self {
        statx_timestamp {
            tv_sec: sec,
            tv_nsec: nsec as _,
            __statx_timestamp_pad1: [0],
        }
    }
}
