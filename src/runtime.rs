use driver::Driver;
use executor::Executor;
use std::cell::{RefCell, RefMut};
use std::future::Future;
use std::rc::Rc;

mod driver;
mod executor;

/// The osiris local runtime.
/// For the moment it cannot be customized.
#[derive(Clone)]
pub struct Runtime(Rc<RefCell<Inner>>);

pub struct Inner {
    driver: Driver,
    executor: Executor,
}

impl Runtime {
    pub fn new() -> Runtime {
        Runtime(Rc::new(RefCell::new(Inner {
            driver: Driver {},
            executor: Executor::new(),
        })))
    }
    /// Runs a future to completion in the current thread.
    pub fn block_on<F>(&self, future: F) -> F::Output
    where
        F: Future,
    {
        todo!()
    }

    /// get a mutable reference to inside of the runtime
    fn get(&self) -> RefMut<Inner> {
        self.0.borrow_mut()
    }
}
