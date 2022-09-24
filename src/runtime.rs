use driver::Driver;
use executor::Executor;
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

use crate::task::JoinHandle;

mod driver;
mod executor;
pub(crate) mod waker;

thread_local! {
    static RUNTIME: RefCell<Option<Runtime>>= RefCell::new(None);
}

pub fn current() -> Option<Runtime> {
    RUNTIME.with(|cell| cell.borrow().clone())
}
pub(crate) fn current_unwrap(fun: &str) -> Runtime {
    if let Some(rt) = current() {
        return rt;
    }
    panic!("called `{fun}` from the outside of a runtime context.")
}

/// The osiris local runtime.
/// For the moment it cannot be customized.
#[derive(Clone)]
pub struct Runtime(Rc<Inner>);

pub struct Inner {
    _driver: Driver,
    executor: Executor,
}

impl Runtime {
    pub fn new() -> Runtime {
        Runtime(Rc::new(Inner {
            _driver: Driver {},
            executor: Executor::new(),
        }))
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
    {
        let task = self.0.executor.spawn(future);
        JoinHandle::new(task)
    }

    /// Runs a future to completion in the current thread.
    pub fn block_on<F>(&self, future: F) -> F::Output
    where
        F: Future,
    {
        self.0.executor.block_on(future)
    }
}
