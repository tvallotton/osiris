#![allow(warnings)]

use io_uring::{cqueue, squeue, IoUring};
use std::borrow::BorrowMut;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::io;
use std::ops::ControlFlow;
use std::ops::ControlFlow::*;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::task::{Poll, Waker};
use std::time::Duration;

use crate::detach;
use crate::runtime::Config;
use crate::time::sleep;
use crate::utils::{epoll_event, syscall};

pub mod event;
pub mod op;

#[non_exhaustive]
pub(crate) struct Driver {
    // pub(crate) epoll: OwnedFd,
    /// the wakers for tasks listening for IO.
    pub(crate) wakers: HashMap<u64, ControlFlow<cqueue::Entry, Waker>>,
    /// this value corresponds to the last occupied id.
    /// This id will be stored in io-uring's `user_data` attribute
    event_id: u64,
    io_uring: IoUring,
}

#[allow(warnings)]
impl Driver {
    /// creates a new driver.
    #[allow(unused_variables)]
    pub fn new(config: Config) -> io::Result<Driver> {
        #[cfg(target_os = "linux")]
        let wakers = HashMap::with_capacity(config.init_capacity);
        #[cfg(target_os = "linux")]
        let io_uring = config.io_uring()?;
        let event_id = 0;
        let driver = Driver {
            wakers,
            event_id: 1,
            io_uring,
        };
        Ok(driver)
    }

    pub fn submit_and_yield(&mut self) -> io::Result<()> {
        self.io_uring.submit()?;
        self.wake_tasks();
        Ok(())
    }

    pub fn submit_and_wait(&mut self) -> io::Result<()> {
        self.io_uring.submit_and_wait(1)?;
        self.wake_tasks();
        Ok(())
    }

    pub fn wake_tasks(&mut self) {
        let cqueue = self.io_uring.completion();
        for cevent in cqueue {
            let Entry::Occupied(mut entry) = self.wakers.entry(cevent.user_data()) else {
                unreachable!(
                        "This is a bug in osiris: a waker has been lost, a CQE was recieved but no associated waker was found."
                    );
            };
            let Continue(waker) = entry.insert(Break(cevent)) else {
                unreachable!(
                        "This is a bug in osiris: a non-multishot SQE has recieved more than one associated CQE."
                    );
            };
            waker.wake();
        }
    }

    #[inline]
    pub fn event_id(&mut self) -> u64 {
        self.event_id += 1;
        self.event_id
    }

    #[inline]
    pub fn poll(&mut self, id: u64, waker: &Waker) -> Poll<cqueue::Entry> {
        let mut entry = self.wakers.entry(id);

        match entry {
            Entry::Vacant(entry) => {
                entry.insert(ControlFlow::Continue(waker.clone()));
                Poll::Pending
            }
            Entry::Occupied(mut entry) => {
                let ControlFlow::Break(_) = entry.get_mut() else {
                    entry.insert(ControlFlow::Continue(waker.clone()));
                    return Poll::Pending;
                };
                let ControlFlow::Break(ready) = entry.remove() else {
                    unreachable!()
                };

                Poll::Ready(ready)
            }
        }
    }

    /// Attempts to push an entry into the queue, returning an available id
    /// for the entry.
    /// If the queue is full, an error is returned.
    ///
    /// # Safety
    ///
    /// Developers must ensure that parameters of the entry (such as buffer) are valid and will
    /// be valid for the entire duration of the operation, otherwise it may cause memory problems.
    pub unsafe fn push(&mut self, entry: squeue::Entry) -> std::io::Result<u64> {
        let id = self.event_id();
        let entry = entry.user_data(id);

        let mut queue = self.io_uring.submission();

        if queue.is_full() {
            drop(queue);
            self.io_uring.submit()?;
            // Safety: Invariants must be upheld by the caller.
            unsafe { self.io_uring.submission().push(&entry) };
        } else {
            // Safety: Invariants must be upheld by the caller.
            unsafe { queue.push(&entry) };
            drop(queue);
        }
        Ok(id)
    }
}
