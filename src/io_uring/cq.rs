use std::{
    fmt::Debug,
    iter::from_fn,
    sync::atomic::{AtomicU32, Ordering},
};

use super::bindings;

pub struct Completion {
    pub head: *mut AtomicU32,
    pub tail: *mut AtomicU32,
    pub flags: *mut AtomicU32,
    // I have no idea what this is for
    pub overflow: *mut AtomicU32,
    pub cqes: *mut Entry,
    pub ring_mask: u32,
    pub ring_entries: u32,
}

impl Debug for Completion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Completion")
            .field("head", &self.head().load(Ordering::Relaxed))
            .field("tail", &self.tail().load(Ordering::Relaxed))
            .field("flags", &self.flags().load(Ordering::Relaxed))
            .field("overflow", &self.overflow().load(Ordering::Relaxed))
            .field("cqes", &self.peek().collect::<Vec<_>>())
            .finish()
    }
}

impl Completion {
    #[inline]
    pub fn head(&self) -> &AtomicU32 {
        // Safety: the value is always initialized
        unsafe { &*self.head }
    }

    #[inline]
    pub fn tail(&self) -> &AtomicU32 {
        // Safety: the value is always initialized
        unsafe { &*self.tail }
    }
    #[inline]
    pub fn flags(&self) -> &AtomicU32 {
        // Safety: the value is always initialized
        unsafe { &*self.flags }
    }
    #[inline]
    pub fn overflow(&self) -> &AtomicU32 {
        // Safety: the value is always initialized
        unsafe { &*self.overflow }
    }

    /// Returns an iterator for the completion queue. The viewed values will be commited to
    /// the submission queue when the iterator gets dropped. This does not mean that the kernel
    /// will be notified about it. It only means that the atomic operation will be performed.
    pub unsafe fn iter(&mut self) -> impl Iterator<Item = Entry> + '_ {
        let head = self.head().load(Ordering::Acquire);
        let tail = self.tail().load(Ordering::Relaxed);
        let mask = self.ring_mask;

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
            cq: self,
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

    pub fn peek(&self) -> impl Iterator<Item = Entry> + '_ {
        let mut head = self.head().load(Ordering::Acquire);
        let tail = self.tail().load(Ordering::Relaxed);
        let mask = self.ring_mask;
        from_fn(move || {
            if head == tail {
                return None;
            } // bounds check
            if head >= self.ring_entries {
                head = 0;
            }

            let index = head & mask;

            // Safety: this must always be in bounds because of the check above
            let cq = unsafe { *self.cqes.offset(index as isize) };

            head += 1;
            Some(cq)
        })
    }
}

pub use bindings::io_uring_cqe as Entry;
