use std::{collections::HashMap, io, task::Waker};

use crate::{hasher::NoopHasher, runtime::Config};

use super::pollster::Pollster;

#[non_exhaustive]
pub(crate) struct Driver {
    /// the wakers for tasks listening for IO.
    wakers: HashMap<u64, Waker, NoopHasher>,
    /// this value always corresponds to an available id.
    /// This id will be stored in io-uring's `user_data` attribute
    event_id: u64,
    pollster: Pollster,
}

impl Driver {
    pub fn new(config: Config) -> io::Result<Driver> {
        let wakers = {
            let hasher = NoopHasher::default();
            HashMap::with_capacity_and_hasher(Config::DEFAULT_WAKERS, hasher)
        };
        let pollster = Pollster::new(config)?;
        let event_id = 0;
        let driver = Driver {
            wakers,
            event_id,
            pollster,
        };
        Ok(driver)
    }

    pub fn submit_and_yield(&mut self) -> io::Result<()> {
        match &mut self.pollster {
            Pollster::IoUring(ring) => ring.submit()?,
        };
        Ok(())
    }

    pub fn submit_and_wait(&mut self) -> io::Result<()> {
        match &mut self.pollster {
            Pollster::IoUring(ring) => ring.submit_and_wait(1)?,
        };
        Ok(())
    }
    // /// Submits an sqe entry, and will call the waker provided when it is ready.
    // ///
    // /// # Safety
    // ///
    // /// Developers must ensure that parameters of the entry (such as buffer) are valid and will
    // /// be valid for the entire duration of the operation, otherwise it may cause memory problems.
    // unsafe fn submit_io(&self, mut entry: Entry, waker: Waker) -> std::io::Result<u64> {
    //     let id = self.event_id();
    //     entry = entry.user_data(id);
    //     self.wakers.borrow_mut().insert(id, waker);
    //     // Safety:
    //     // the validity of the entry is upheld by the caller.
    //     unsafe { self.pollster.borrow_mut().sumit_io(entry)? };
    //     Ok(id)
    // }
    // /// Updates a waker for a specified `event_id`.
    // fn update_waker(&self, event_id: u64, waker: Waker) -> Poll<()> {
    //     use std::collections::hash_map::Entry;
    //     let mut wakers = self.wakers.borrow_mut();
    //     if let Entry::Occupied(mut entry) = wakers.entry(event_id) {
    //         entry.insert(waker);
    //         Poll::Pending
    //     } else {
    //         Poll::Ready(())
    //     }
    // }

    // pub fn poll(&mut self) {
    //     let Self {
    //         wakers, pollster, ..
    //     } = self;
    //     for event_id in pollster.woken() {
    //         if let Some(waker) = wakers.remove(&event_id) {
    //             waker.wake();
    //         }
    //     }
    // }
}
