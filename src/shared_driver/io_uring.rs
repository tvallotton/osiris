use io_uring::{opcode, squeue::Entry, Builder, IoUring};

use crate::runtime::{config::Config, unique_queue::NoopHasher, waker};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, VecDeque},
    future::poll_fn,
    io,
    mem::ManuallyDrop,
    task::{Poll, Waker},
    time::Duration,
};
const DEFAULT_ENTRIES: u32 = 2048;
const DEFAULT_WAKERS: usize = 2048;

pub(crate) struct Driver {
    event_id: Cell<u64>,
    waiters: RefCell<HashMap<u64, Waker, NoopHasher>>,
    pub(crate) io_uring: RefCell<IoUring>,
}

impl Driver {
    pub fn new(config: Config) -> io::Result<Driver> {
        let builder = IoUring::builder();
        let event_id = Cell::new(0);
        let io_uring = builder.build(config.io_uring_entries)?;
        let io_uring = RefCell::new(io_uring);
        let waiters = HashMap::with_capacity_and_hasher(DEFAULT_WAKERS, NoopHasher(0));
        let waiters = RefCell::new(waiters);
        let driver = Driver {
            io_uring,
            event_id,
            waiters,
        };
        Ok(driver)
    }
    pub fn submit_yield(&self) -> std::io::Result<()> {
        self.io_uring.borrow().submit()?;
        Ok(())
    }

    pub fn submit_wait(&self) -> io::Result<()> {
        self.io_uring.borrow_mut().submit_and_wait(1)?;
        Ok(())
    }

    pub async unsafe fn push<T>(&mut self, entry: Entry, data: T) -> T {
        let mut data = ManuallyDrop::new(Some(data));
        let event_id = self.event_id.get();
        self.event_id.set(event_id.overflowing_add(1).0);

        let entry = entry.user_data(event_id);

        poll_fn(move |cx| Poll::Ready(data.take().expect("future polled after ready."))).await
    }

    fn poll(&mut self) {
        let waiters = self.waiters.borrow_mut();
        self.io_uring
            .borrow_mut()
            .completion()
            .map(|entry| entry.user_data())
            .for_each(|waker_id| {
                if let Some(waker) = waiters.remove(&waker_id) {
                    waker.wake();
                }
            })
    }
}
