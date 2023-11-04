use std::cell::{RefCell, RefMut};
use std::io;
use std::rc::Rc;
#[cfg(io_uring)]
use ::{
    io_uring::{cqueue, squeue::Entry},
    std::task::{Context, Poll},
};

#[cfg(io_uring)]
pub(crate) use iouring::{op, Driver};

#[cfg(not(io_uring))]
pub(crate) use poll::{op, Driver, Event};

#[cfg(kqueue)]
pub(crate) use kqueue::{op, Driver};

use crate::runtime::Config;

// #[cfg(target_os = "linux")]
// mod epoll;
#[cfg(io_uring)]
mod iouring;

#[cfg(target_os = "macos")]
mod kqueue;

#[cfg(not(io_uring))]
mod nonblocking;

#[cfg(not(io_uring))]
mod poll;

// mod wakerstore;

/// The driver stores the wakers for all the tasks that
/// are waiting for IO and it will wake them when it is
#[derive(Clone)]
pub(crate) struct Reactor(Rc<RefCell<Driver>>);
impl Reactor {
    // creates a new shared driver.
    pub fn new(config: Config) -> io::Result<Self> {
        let driver = Driver::new(config)?;
        let driver = RefCell::new(driver);
        let driver = Rc::new(driver);
        Ok(Self(driver))
    }
    /// submits all io-events to the kernel and waits for at least one completion event.
    pub fn submit_and_wait(&self) -> io::Result<()> {
        let mut reactor = self.0.borrow_mut();
        reactor.submit_and_wait()?;
        reactor.wake_tasks();
        Ok(())
    }
    /// submits all io-events to the kernel and yields immediately without blocking the thread.
    pub fn submit_and_yield(&self) -> io::Result<()> {
        let mut reactor = self.0.borrow_mut();
        reactor.submit_and_yield()?;
        reactor.wake_tasks();
        Ok(())
    }

    pub fn driver(&self) -> RefMut<'_, Driver> {
        self.0.borrow_mut()
    }

    /// This function is used to poll the driver about a specific event.
    ///
    /// When polled, the driver will update the waker for the IO event, and
    /// will either return pending, or the `cqueue::Entry` if the event is ready.
    #[cfg(io_uring)]
    #[inline]
    pub fn poll(&self, id: u64, cx: &mut Context) -> Poll<cqueue::Entry> {
        self.0.borrow_mut().poll(id, cx.waker())
    }

    /// Attempts to push an entry into the queue.
    /// If the queue is full, an error is returned.
    ///
    /// # Safety
    ///
    /// Developers must ensure that parameters of the entry (such as buffer) are valid and will
    /// be valid for the entire duration of the operation, otherwise it may cause memory problems.
    #[cfg(io_uring)]
    pub unsafe fn push(&self, entry: Entry) -> std::io::Result<u64> {
        // Safety: Invariants must be upheld by the caller.
        unsafe { self.0.borrow_mut().push(entry) }
    }
}
fn current() -> Reactor {
    const ERR_MSG: &str =
        "attempted to perform async I/O from the outside of an osiris runtime context.";
    crate::runtime::current().expect(ERR_MSG).reactor
}
