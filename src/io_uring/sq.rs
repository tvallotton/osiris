use super::bindings::{self, IORING_SQ_NEED_WAKEUP};

use std::{
    fmt::Debug,
    sync::atomic::{AtomicU32, Ordering},
};

pub struct Submission {
    pub head: *mut AtomicU32,
    pub tail: *mut AtomicU32,
    pub ring_mask: u32,
    pub ring_entries: u32,
    pub flags: *mut AtomicU32,
    pub dropped: *mut AtomicU32,
    pub array: *mut u32,
    pub sqes: *mut bindings::io_uring_sqe,
}

impl Debug for Submission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Submission")
            .field("head", &self.head().load(Ordering::Relaxed))
            .field("tail", &self.tail().load(Ordering::Relaxed))
            .field("ring_mask", &self.ring_mask)
            .field("ring_mask", &self.ring_entries)
            .field("dropped", &self.dropped().load(Ordering::Relaxed))
            .field("array", &self.array())
            .field("needs_wakeup", &self.needs_wakeup())
            .finish()
    }
}

impl Submission {
    #[inline]
    pub fn head(&self) -> &AtomicU32 {
        // Safety: the reference is aways valid
        unsafe { &*self.head }
    }
    #[inline]
    pub fn tail(&self) -> &AtomicU32 {
        // Safety: the reference is always valid
        unsafe { &*self.tail }
    }
    #[inline]
    pub fn dropped(&self) -> &AtomicU32 {
        // Safety: the reference is always valid
        unsafe { &*self.dropped }
    }
    #[inline]
    pub fn flags(&self) -> &AtomicU32 {
        // Safety: the reference is always valid
        unsafe { &*self.flags }
    }

    #[inline]
    pub fn needs_wakeup(&self) -> bool {
        (self.flags().load(Ordering::Relaxed) & IORING_SQ_NEED_WAKEUP) != 0
    }

    pub fn array(&self) -> &[AtomicU32] {
        // Safety: Not really
        unsafe { std::slice::from_raw_parts(self.array.cast(), self.ring_entries as usize) }
    }
    /// # Safety
    /// all reasources from the entry must outlive the cqe.
    /// That is, they must be 'static.
    pub unsafe fn push(&self, _entry: Entry) {
        todo!()
        // let tail = &mut *self.tail;
        // let next_tail = self.tail.offset(1);
        // fence(Ordering::Acquire);
        // let index = tail & *self.ring_mask;
    }
}

pub use bindings::io_uring_sqe as Entry;
