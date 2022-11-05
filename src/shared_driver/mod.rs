use self::driver::Driver;
use crate::runtime::Config;
#[cfg(target_os = "linux")]
use io_uring::squeue::Entry;
use std::cell::RefCell;
use std::io;
use std::rc::Rc;
mod driver;
mod event;

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
    #[cfg(target_os = "linux")]
    pub unsafe fn push(&self, entry: &Entry) -> std::io::Result<()> {
        // Safety: Invariants must be upheld by the caller.
        unsafe { self.0.borrow_mut().push(entry) }
    }
}
