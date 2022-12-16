#![allow(warnings)]
use bindings::IoUringParams;
use bindings::*;
use std::mem::size_of;
use std::ptr::{null, null_mut};

use crate::io_uring::syscall::allocate_ring;

use self::cq::Cq;
use self::sq::Sq;

mod bindings;

mod config;
mod cq;
mod sq;
mod syscall;

struct IoUring {
    ring_fd: i32,
    sq: sq::Sq,
    sqes: *mut bindings::io_uring_sqe,
    cq: cq::Cq,
}

impl IoUring {
    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn new(depth: u32, params: IoUringParams) -> std::io::Result<Self> {
        let ring_fd = syscall::io_uring_setup(depth, &params)?;

        let mut ssize = params.sq_off.array + params.sq_entries * size_of::<u32>() as u32;
        let mut csize =
            params.cq_off.cqes + params.cq_entries * size_of::<bindings::io_uring_cqe>() as u32;

        // In kernel version 5.4 and above, it is possible to map the submission and
        // completion buffers with a single mmap() call. Rather than check for kernel
        // versions, the recommended way is to just check the features field of the
        // io_uring_params structure, which is a bit mask. If the
        // IORING_FEAT_SINGLE_MMAP is set, then we can do away with the second mmap()
        // call to map the completion ring.
        if params.feat_single_map() {
            csize = ssize.max(csize);
            ssize = csize;
        }

        let sq_ptr = allocate_ring(ring_fd, ssize as _, bindings::IORING_OFF_SQ_RING);

        let mut cq_ptr = null_mut();
        if params.feat_single_map() {
            cq_ptr = sq_ptr;
        } else {
            cq_ptr = allocate_ring(ring_fd, ssize as _, bindings::IORING_OFF_CQ_RING);
        }
        let sq = Sq {
            head: sq_ptr.offset(params.sq_off.head()),
            tail: sq_ptr.offset(params.sq_off.tail()),
            ring_mask: sq_ptr.offset(params.sq_off.ring_mask()),
            ring_entries: sq_ptr.offset(params.sq_off.ring_entries()),
            flags: sq_ptr.offset(params.sq_off.flags()),
            array: sq_ptr.offset(params.sq_off.array()),
        };
        let sqes_size = params.sq_entries as usize * size_of::<io_uring_sqe>();
        let sqes = allocate_ring(ring_fd, sqes_size, bindings::IORING_OFF_SQES);

        let cq = Cq {
            head: cq_ptr.offset(params.cq_off.head()),
            tail: cq_ptr.offset(params.cq_off.tail()),
            ring_mask: cq_ptr.offset(params.cq_off.ring_mask()),
            ring_entries: cq_ptr.offset(params.cq_off.ring_entries()),
            cqes: cq_ptr.offset(params.cq_off.cqes()).cast(),
        };

        todo!()
    }
}

// fn io_uring_setup(entries: u32, params: &IoUringParams) -> i32 {
//     // Safety:
//     unsafe { syscall(libc::SYS_io_uring_setup, entries, params) as i32 }
// }
// unsafe fn io_uring_enter(ring_fd: i32, to_submit: u32, min_complete: u32, flags: u32) -> i64 {
//     // Safety: upheld by the caller
//     unsafe {
//         syscall(
//             libc::SYS_io_uring_enter,
//             ring_fd,
//             to_submit,
//             min_complete,
//             flags,
//             0,
//             0,
//         )
//     }
// }
