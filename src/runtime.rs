use driver::Driver;
use executor::Executor;
use std::cell::RefCell;
use std::future::Future;
use std::panic::Location;
use std::rc::Rc;

use crate::task::JoinHandle;

mod driver;
mod executor;
mod unique_queue;
pub(crate) mod waker;

thread_local! {
    static RUNTIME: RefCell<Option<Runtime>>= RefCell::new(None);
}

/// Returns a handle to the currently running [`Runtime`].
/// # Panics
/// This will panic if called outside the context of a osiris runtime.
/// It is ok to call this function from a spawned task or from a [blocked on](block_on) future. 
#[track_caller]
pub fn current() -> Runtime {
    if let Some(runtime) = Runtime::current() {
        return runtime 
    }
    panic!("called `current` from the outside of a runtime context.")
}

pub fn block_on<F: Future>(f: F) -> F::Output {
    Runtime::new().block_on(f)
}


#[track_caller]
pub(crate) fn current_unwrap(fun: &str) -> Runtime {
    if let Some(rt) = Runtime::current() {
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

    pub fn current() -> Option<Runtime> {
        RUNTIME.with(|cell| cell.borrow().clone())
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
    {
        let task = self.0.executor.spawn(future);

        JoinHandle::new(task)
    }

    /// Runs a future to completion on the osiris runtime. This is the
    /// runtime's entry point.
    ///
    /// This runs the given future on the current thread, blocking until it is
    /// complete, and yielding its resolved result. Any tasks or timers
    /// which the future spawns internally will be executed on the runtime.
    ///
    /// # Current thread scheduler
    ///
    /// When the current thread scheduler is enabled `block_on`
    /// can be called concurrently from multiple threads. The first call
    /// will take ownership of the io and timer drivers. This means
    /// other threads which do not own the drivers will hook into that one.
    /// When the first `block_on` completes, other threads will be able to
    /// "steal" the driver to allow continued execution of their futures.
    ///
    /// Any spawned tasks will be suspended after `block_on` returns. Calling
    /// `block_on` again will resume previously spawned tasks.
    ///
    /// # Panics
    ///
    /// This function panics if the provided future panics, or if called within an
    /// asynchronous execution context.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use osiris::runtime::Runtime;
    ///
    /// // Create the runtime
    /// let rt  = Runtime::new();
    ///
    /// // Execute the future, blocking the current thread until completion
    /// rt.block_on(async {
    ///     println!("hello");
    /// });
    /// ```
    ///
    /// [handle]: fn@Handle::block_on
    #[track_caller]
    pub fn block_on<F>(&self, future: F) -> F::Output
    where
        F: Future,
    {
        assert!(
            Runtime::current().is_none(),
            "called `block_on` from the inside of another osiris runtime."
        );
        let _h = self.enter();
        self.0.executor.block_on(future)
    }

    pub fn enter(&self) -> impl Drop + '_ {
        struct Enter<'a>(Option<Runtime>, &'a Runtime);
        impl<'a> Drop for Enter<'a> {
            fn drop(&mut self) {
                RUNTIME.with(|cell| cell.replace(self.0.take()));
            }
        }
        let new_rt = Some(self.clone());
        let rt = RUNTIME.with(|cell| cell.replace(new_rt));
        Enter(rt, &self)
    }
}
