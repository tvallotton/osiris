use super::bindings::{self, IoUringParams};
use libc::*;
use std::{io, ptr::null_mut};

macro_rules! syscall {
    ($fun:ident, $($args:expr),* $(,)?) => {{
        let res = libc::syscall(libc::$fun, $($args),*);
        if res < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(res)
        }
    }};
}

pub unsafe fn allocate_ring(ring_fd: i32, size: usize, side: u32) -> *mut u32 {
    let ptr = unsafe {
        libc::mmap(
            null_mut(),
            size,
            PROT_READ | PROT_WRITE,
            MAP_SHARED | MAP_POPULATE,
            ring_fd,
            side as _,
        )
    };
    if ptr == libc::MAP_FAILED {
        panic!("failed to allocate io-uring buffers")
    }
    ptr as _
}

pub fn io_uring_setup(entries: u32, params: &IoUringParams) -> io::Result<i32> {
    // Safety:
    let fd = unsafe { syscall!(SYS_io_uring_setup, entries, params)? };
    Ok(fd as i32)
}
pub unsafe fn io_uring_enter(
    ring_fd: i32,
    to_submit: u32,
    min_complete: u32,
    flags: u32,
) -> io::Result<i32> {
    // Safety: upheld by the caller
    let fd = unsafe {
        syscall!(
            SYS_io_uring_enter,
            ring_fd,
            to_submit,
            min_complete,
            flags,
            0,
            0,
        )?
    };
    Ok(fd as i32)
}
