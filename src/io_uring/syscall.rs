use super::config::Params;

use libc::*;
use std::{io, ptr::null_mut};

/// Safety:
/// * ring_fd must be a valid file descriptor initialized from an io_uring_setup system call
/// * size must be the size in bytes of the queue to be queue being allocated as returned by the
///     io_uring_setup syscall
/// * side must be one of IORING_OFF_CQ_RING | IORING_OFF_SQES | IORING_OFF_SQ_RING
pub unsafe fn allocate_queue(ring_fd: i32, size: usize, side: u32) -> *mut u32 {
    // Safety: guaranteed by the caller
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

/// # Safety
/// * the number of entries must be a a power of two smaller or equal to `1 << 12`.
pub unsafe fn io_uring_setup(entries: u32, params: &mut Params) -> io::Result<i32> {
    // Safety: guaranteed by the caller
    let fd = unsafe { syscall(SYS_io_uring_setup, entries, params) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(fd as i32)
}

/// # Safety
/// * `fd` must be a valid io-uring file descriptor
pub unsafe fn io_uring_enter(
    ring_fd: i32,
    to_submit: u32,
    min_complete: u32,
    flags: u32,
) -> io::Result<i32> {
    // Safety: upheld by the caller
    let fd = unsafe {
        syscall(
            SYS_io_uring_enter,
            ring_fd,
            to_submit,
            min_complete,
            flags,
            0,
            0,
        )
    };
    if fd < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(fd as i32)
    }
}
