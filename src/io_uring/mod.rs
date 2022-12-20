// #![allow(warnings)]

use config::Params;
#[allow(warnings)]
use std::iter::from_fn;
use std::sync::atomic::Ordering;

use crate::io_uring::syscall::allocate_queue;

use self::bindings::{
    IORING_ENTER_GETEVENTS, IORING_ENTER_SQ_WAKEUP, IORING_OFF_CQ_RING, IORING_OFF_SQES,
    IORING_OFF_SQ_RING, IORING_SETUP_SQPOLL,
};
use self::cq::Completion;
use self::sq::Submission;

pub mod bindings;

mod config;
mod cq;
mod sq;

mod syscall;

#[derive(Debug)]
struct IoUring {
    /// File descriptor for this ring
    fd: i32,
    /// Submission queue
    sq: sq::Submission,
    /// Completion queue
    cq: cq::Completion,
    /// The parameters used to setup this ring
    params: Params,
}

impl Drop for IoUring {
    fn drop(&mut self) {
        // Safety: the file descriptor is aways valid
        if unsafe { libc::close(self.fd) < 0 } {
            Err(std::io::Error::last_os_error()).unwrap()
        }
    }
}

impl IoUring {
    #[allow(unsafe_op_in_unsafe_fn)]
    fn new(depth: u32, mut params: Params) -> std::io::Result<Self> {
        // Safety:
        unsafe {
            let fd = syscall::io_uring_setup(depth, &mut params)?;

            let mut sq_size = params.sq_size();
            let mut cq_size = params.cq_size();

            if params.feat_single_allocation() {
                cq_size = sq_size.max(cq_size);
                sq_size = cq_size;
            }

            let sq_ptr = allocate_queue(fd, sq_size as _, IORING_OFF_SQ_RING);

            let cq_ptr = {
                if params.feat_single_allocation() {
                    sq_ptr
                } else {
                    allocate_queue(fd, cq_size as _, IORING_OFF_CQ_RING)
                }
            };
            let sqes = allocate_queue(fd, params.sqes_size(), IORING_OFF_SQES).cast();

            let sq = Submission {
                head: sq_ptr.offset(params.sq_off.head()).cast(),
                tail: sq_ptr.offset(params.sq_off.tail()).cast(),
                ring_mask: *sq_ptr.offset(params.sq_off.ring_mask()),
                ring_entries: *sq_ptr.offset(params.sq_off.ring_entries()),
                flags: sq_ptr.offset(params.sq_off.flags()).cast(),
                array: sq_ptr.offset(params.sq_off.array()),
                dropped: sq_ptr.offset(params.sq_off.dropped()).cast(),
                sqes,
            };

            let cq = Completion {
                head: cq_ptr.offset(params.cq_off.head()).cast(),
                tail: cq_ptr.offset(params.cq_off.tail()).cast(),
                ring_mask: *cq_ptr.offset(params.cq_off.ring_mask()),
                ring_entries: *cq_ptr.offset(params.cq_off.ring_entries()),
                cqes: cq_ptr.offset(params.cq_off.cqes()).cast(),
                flags: cq_ptr.offset(params.cq_off.flags()).cast(),
                overflow: cq_ptr.offset(params.cq_off.overflow()).cast(),
            };

            Ok(Self { fd, sq, cq, params })
        }
    }

    /// # Safety
    /// all resources from the entry must outlive the cqe.
    /// That is, they must be 'static.
    pub unsafe fn push(&mut self, entry: sq::Entry) {
        // Safety: guaranteed by the caller
        unsafe { self.sq.push(entry) }
    }

    /// Returns an iterator for the completion queue. The viewed values will be commited to
    /// the submission queue when the iterator gets dropped. This does not mean that the kernel
    /// will be notified about it. It only means that the atomic operation will be performed.
    pub unsafe fn iter(&mut self) -> impl Iterator<Item = cq::Entry> + '_ {
        let head = self.cq.head().load(Ordering::Acquire);
        let tail = self.cq.tail().load(Ordering::Relaxed);
        let mask = self.cq.ring_mask;

        /// This guard will commit the reads when the iterator is dropped.
        struct Guard<'a> {
            head: u32,
            tail: u32,
            cq: &'a mut Completion,
        }

        impl<'a> Drop for Guard<'a> {
            fn drop(&mut self) {
                self.cq.head().store(self.head, Ordering::Release);
            }
        }
        let mut s = Guard {
            head,
            tail,
            cq: &mut self.cq,
        };

        from_fn(move || {
            // There is data available in the ring buffer
            if s.head == s.tail {
                return None;
            }
            // bounds check
            if s.head >= s.cq.ring_entries {
                s.head = 0;
            }

            let index = s.head & mask;

            // Safety: this must always be in bounds because of the check above
            let cq = unsafe { *s.cq.cqes.offset(index as isize) };
            s.head += 1;
            Some(cq)
        })
    }

    pub fn submit_and_yield(&mut self) -> std::io::Result<()> {
        self.submit_and_wait(0)
    }

    pub fn submit_and_wait(&mut self, events: u32) -> std::io::Result<()> {
        // Safety:
        unsafe { syscall::io_uring_enter(self.fd, self.to_submit(), events, self.submit_flags())? };
        Ok(())
    }

    pub fn submit_flags(&self) -> u32 {
        if self.poll_mode() && self.sq.needs_wakeup() {
            IORING_ENTER_SQ_WAKEUP | IORING_ENTER_GETEVENTS
        } else {
            IORING_ENTER_GETEVENTS
        }
    }

    pub fn poll_mode(&self) -> bool {
        (self.params.flags & IORING_SETUP_SQPOLL) != 0
    }

    pub fn to_submit(&self) -> u32 {
        todo!()
    }
}

fn normalize_size(x: u64) -> u32 {
    x.next_power_of_two().min(1 << 12) as _
}

#[test]
fn foo() {
    let params = Default::default();
    let io_uring = IoUring::new(8, params).unwrap();

    println!("{:#?}", io_uring);
    println!("{:#?}", io_uring.params.feat_single_allocation())
}
