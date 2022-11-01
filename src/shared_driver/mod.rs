use self::driver::Driver;
use crate::runtime::Config;
use std::cell::RefCell;
use std::io;
use std::rc::Rc;

mod driver;
mod event;
mod pollster;

#[derive(Clone)]
pub(crate) struct SharedDriver(Rc<RefCell<Driver>>);

impl SharedDriver {
    // creates a new shared driver.
    pub fn new(config: Config) -> io::Result<Self> {
        let driver = Driver::new(config)?;
        let driver = RefCell::new(driver);
        let driver = Rc::new(driver);
        Ok(Self(driver))
    }
    /// submits all io-events to the kernel and waits for at least one completion event.
    pub fn submit_and_wait(&self) -> std::io::Result<()> {
        self.0.borrow_mut().submit_and_wait()?;
        Ok(())
    }
    /// submits all io-events to the kernel and yields immediately without blocking the thread.
    pub fn submit_and_yield(&self) -> std::io::Result<()> {
        self.0.borrow_mut().submit_and_yield()?;
        Ok(())
    }
    /// wakes up any tasks listening for IO events.
    pub fn wake_tasks(&self) {
        self.0.borrow_mut().wake_tasks();
    }
}

// /// Helper function for submitting io-events
// ///
// /// # Safety
// /// The caller of the function must guarantee that the entry and all its
// /// inputs will live through the entire lifetime of the io event. Any buffers
// /// can be safely stored in the `data` input, which will be returned after the operation
// /// is complete.
// ///
// pub async unsafe fn submit_event<T: 'static>(entry: Entry, data: T) -> std::io::Result<T> {
//     let driver =
//         current().expect("attempted to perform an IO event outside an osiris runtime context.");
//     let waker = poll_fn(|cx| Poll::Ready(cx.waker().clone())).await;
//
//     // SAFETY:
//     // the data input is moved into the `task` given to the `complete`
//     // closure, which guarantees that will be driven to completion before it
//     // gets dropped.
//     let event_id = unsafe { driver.submit_io(entry, waker)? };
//
//     let mut data = Some(data);
//     let task = poll_fn(move |cx| {
//         let poll = driver.update_waker(event_id, cx.waker().clone());
//         if poll.is_ready() {
//             return Poll::Ready(data.take().unwrap());
//         }
//         Poll::Pending
//     });
//     Ok(complete(task).await)
// }
//
