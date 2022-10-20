use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::poll_fn;
use std::io;
use std::mem::ManuallyDrop;
use std::rc::Rc;

use crate::hasher::NoopHasher;
use crate::runtime::Config;
use crate::task::complete;
use io_uring::squeue::Entry;
use pollster::Pollster;
use std::task::{Poll, Waker};
mod pollster;

fn current() -> Option<Rc<Driver>> {
    let rt = crate::runtime::current()?;
    Some(rt.0.driver.clone())
}

#[non_exhaustive]
pub(crate) struct Driver {
    /// the wakers for tasks listening for IO.
    wakers: RefCell<HashMap<u64, Waker, NoopHasher>>,
    /// this value always corresponds to an available id.
    /// This id will be stored in io-uring's `user_data` attribute
    event_id: Cell<u64>,
    pollster: RefCell<Pollster>,
}

const DEFAULT_ENTRIES: u32 = 2048;
const DEFAULT_WAKERS: usize = 2048;

/// Helper function for submitting io-events
///
/// # Safety
/// The caller of the function must guarantee that the entry and all its
/// inputs will live through the entire lifetime of the io event. Any buffers
/// can be safely stored in the `data` input, which will be returned after the operation
/// is complete.
///
pub async unsafe fn submit_event<T: 'static>(entry: Entry, data: T) -> T {
    let driver =
        current().expect("attempted to perform an IO event outside an osiris runtime context.");
    let waker = poll_fn(|cx| Poll::Ready(cx.waker().clone())).await;

    // SAFETY:
    // the data input is moved into the `task` given to the `complete`
    // closure, which guarantees that will be driven to completion before it
    // gets dropped.
    let event_id = unsafe { driver.submit_io(entry, waker) };

    let mut data = Some(data);
    let task = poll_fn(move |cx| {
        let poll = driver.update_waker(event_id, cx.waker().clone());
        if poll.is_ready() {
            return Poll::Ready(data.take().unwrap());
        }
        Poll::Pending
    });
    complete(task).await
}

impl Driver {
    pub fn new(config: Config) -> std::io::Result<Driver> {
        let pollster = Pollster::new(config)?;
        let pollster = RefCell::new(pollster);
        let wakers = HashMap::with_capacity_and_hasher(DEFAULT_WAKERS, NoopHasher::default());
        let wakers = RefCell::new(wakers);
        let event_id = Cell::default();
        let driver = Driver {
            wakers,
            event_id,
            pollster,
        };
        Ok(driver)
    }
    /// returns a new event id
    fn event_id(&self) -> u64 {
        let event_id = self.event_id.get();
        self.event_id.set(event_id + 1);
        event_id
    }
    pub fn submit_and_yield(&self) -> io::Result<()> {
        let mut pollster = self.pollster.borrow_mut();
        match &mut *pollster {
            Pollster::IoUring(ring) => ring.submit()?,
        };
        Ok(())
    }

    pub fn submit_and_wait(&self) -> io::Result<()> {
        let mut pollster = self.pollster.borrow_mut();
        match &mut *pollster {
            Pollster::IoUring(ring) => ring.submit_and_wait(1)?,
        };
        Ok(())
    }
    /// Submits an sqe entry, and will call the waker provided when it is ready.
    ///
    /// # Safety
    ///
    /// Developers must ensure that parameters of the entry (such as buffer) are valid and will
    /// be valid for the entire duration of the operation, otherwise it may cause memory problems.
    unsafe fn submit_io(&self, mut entry: Entry, waker: Waker) -> u64 {
        let id = self.event_id();
        entry = entry.user_data(id);
        self.wakers.borrow_mut().insert(id, waker);
        // Safety:
        // the validity of the entry is upheld by the caller.
        unsafe {
            self.pollster.borrow_mut().sumit_io(entry);
        }
        id
    }
    /// Updates a waker for a specified `event_id`.
    fn update_waker(&self, event_id: u64, waker: Waker) -> Poll<()> {
        use std::collections::hash_map::Entry;
        let mut wakers = self.wakers.borrow_mut();
        if let Entry::Occupied(mut entry) = wakers.entry(event_id) {
            entry.insert(waker);
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
    pub fn wake(&self, event_id: u64) {
        let mut wakers = self.wakers.borrow_mut();
        if let Some(waker) = wakers.remove(&event_id) {
            waker.wake();
        }
    }
}
