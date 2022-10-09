use io_uring::{opcode, squeue::Entry, Builder, IoUring};

use crate::runtime::config::Config;
use std::{io, time::Duration};
const DEFAULT_ENTRIES: u32 = 2048;

pub(crate) struct Driver {
    event_id: u64,
    pub(crate) io_uring: IoUring,
}

impl Driver {
    pub fn new(config: Config) -> io::Result<Driver> {
        let builder = IoUring::builder();
        let event_id = 0;
        let io_uring = builder.build(config.io_uring_entries)?;
        let driver = Driver { io_uring, event_id };
        Ok(driver)
    }
    pub fn submit_yield(&self) -> std::io::Result<()> {
        self.io_uring.submit()?;
        Ok(())
    }

    pub fn submit_wait(&self) -> io::Result<()> {
        self.io_uring.submit_and_wait(1)?;
        Ok(())
    }

    pub unsafe fn push<E>(&mut self, entry: Entry) -> Option<u64> {
        let id = self.event_id;
        let entry = entry.user_data(id);
        self.event_id = id.overflowing_add(1).0;
        self.io_uring.submission().push(&entry).ok()?;
        Some(id)
    }

    // fn poll(&mut self) {
    //     self.io_uring.completion().map(|entry| entry.user_data())
    // }
}
