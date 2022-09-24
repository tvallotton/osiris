use driver::Driver;
use executor::Executor;
use std::cell::{RefCell, RefMut};
use std::future::Future;
use std::rc::Rc;

mod driver;
mod executor;
mod waker; 

/// The osiris local runtime.
/// For the moment it cannot be customized.
#[derive(Clone)]
pub struct Runtime(Rc<Inner>);

pub struct Inner {
    driver: Driver,
    executor: Executor,
}

impl Runtime {
    pub fn new() -> Runtime {
        Runtime(Rc::new(Inner {
            driver: Driver {},
            executor: Executor::new(),
        }))
    }
    /// Runs a future to completion in the current thread.
    pub fn block_on<F>(&self, future: F) -> F::Output
    where
        F: Future,
    {
        self.0.executor.block_on(future)
    }
}
