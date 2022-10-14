use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::poll_fn;
use std::io;
use std::mem::ManuallyDrop;

use crate::hasher::NoopHasher;
use crate::runtime::{current, Config};
use io_uring::squeue::Entry;
use pollster::Pollster;
use std::task::{Poll, Waker};
mod pollster;

#[non_exhaustive]
pub(crate) struct Driver {
    /// the waker for tasks listening for IO.
    wakers: RefCell<HashMap<u64, Waker, NoopHasher>>,
    /// this value always corresponds to an available id.
    event_id: Cell<u64>,
    pollster: RefCell<Pollster>,
}

const DEFAULT_ENTRIES: u32 = 2048;
const DEFAULT_WAKERS: usize = 2048;

/// # Safety
/// The caller of the function must guarantee that the entry and all its
/// inputs will live through the entire lifetime of the io event. Any buffers
/// can be stored in the `data` input, which will be retuned intact.
///
pub async unsafe fn submit_event<T>(entry: Entry, data: T) -> T {
    let rt = current();
    let driver = &rt.0.driver;
    let waker = poll_fn(|cx| Poll::Ready(cx.waker().clone())).await;

    // we place it in a manually drop to make sure the resource isn't accidentally
    // dropped until the io-event finishes.
    let data = ManuallyDrop::new(data);
    // we can now
    let event_id = unsafe { driver.submit_io(entry, waker) };

    poll_fn(|cx| {
        // when this returns ready, the io operation will be ready.
        driver.update_waker(event_id, cx.waker().clone())
    })
    .await;

    ManuallyDrop::into_inner(data)
}

pub(crate) async unsafe fn submit<T>(entry: Entry, data: T) -> T {
    let mut data = Some(ManuallyDrop::new(data));
    poll_fn(move |cx| {
        let data = data.take().expect("future polled after completion");
        let data = ManuallyDrop::into_inner(data);
        Poll::Ready(data)
    })
    .await
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
    pub fn event_id(&self) -> u64 {
        let event_id = self.event_id.get();
        self.event_id.set(event_id + 1);
        return event_id;
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

    pub unsafe fn submit_io(&self, mut entry: Entry, waker: Waker) -> u64 {
        let id = self.event_id();
        entry = entry.user_data(id);
        self.wakers.borrow_mut().insert(id, waker);
        unsafe {
            self.pollster.borrow_mut().sumit_io(entry);
        }
        id
    }
    pub fn update_waker(&self, event_id: u64, waker: Waker) -> Poll<()> {
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
