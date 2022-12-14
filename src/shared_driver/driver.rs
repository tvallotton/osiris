#[cfg(target_os = "linux")]
use io_uring::{squeue::Entry, IoUring};
use std::collections::HashMap;
use std::io;
use std::task::Waker;

use crate::runtime::Config;

#[non_exhaustive]
pub(crate) struct Driver {
    /// the wakers for tasks listening for IO.
    wakers: HashMap<u64, Waker>,
    /// this value always corresponds to an available id.
    /// This id will be stored in io-uring's `user_data` attribute
    event_id: u64,
    #[cfg(target_os = "linux")]
    io_uring: IoUring,
}
#[allow(warnings)]
impl Driver {
    /// creates a new driver.
    #[allow(unused_variables)]
    pub fn new(config: Config) -> io::Result<Driver> {
        let wakers = { HashMap::with_capacity(config.init_capacity) };
        #[cfg(target_os = "linux")]
        let io_uring = config.io_uring()?;

        let event_id = 0;
        let driver = Driver {
            wakers,
            event_id: 0,
            #[cfg(target_os = "linux")]
            io_uring,
        };
        Ok(driver)
    }

    pub fn submit_and_yield(&mut self) -> io::Result<()> {
        #[cfg(target_os = "linux")]
        self.io_uring.submit()?;
        Ok(())
    }

    pub fn submit_and_wait(&mut self) -> io::Result<()> {
        #[cfg(target_os = "linux")]
        self.io_uring.submit_and_wait(1)?;
        Ok(())
    }
    // TODO
    pub fn wake_tasks(&mut self) {
        let _ = self;
    }

    /// Attempts to push an entry into the queue.
    /// If the queue is full, an error is returned.
    ///
    /// # Safety
    ///
    /// Developers must ensure that parameters of the entry (such as buffer) are valid and will
    /// be valid for the entire duration of the operation, otherwise it may cause memory problems.
    #[cfg(target_os = "linux")]
    pub unsafe fn push(&mut self, entry: &Entry) -> std::io::Result<()> {
        let mut queue = self.io_uring.submission();

        if queue.is_full() {
            drop(queue);
            self.io_uring.submit()?;
            // Safety: Invariants must be upheld by the caller.
            unsafe { self.io_uring.submission().push(entry).ok() };
            Ok(())
        } else {
            // Safety: Invariants must be upheld by the caller.
            unsafe { queue.push(entry).ok() };
            Ok(())
        }
    }
}
