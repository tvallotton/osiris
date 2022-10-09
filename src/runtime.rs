use driver::Driver;
use executor::Executor;
use std::cell::RefCell;
use std::future::{self, Future};
use std::io;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{ready, Context, Poll};

use crate::pin;
use crate::runtime::waker::waker;
use crate::task::JoinHandle;

use self::config::Config;

mod config;
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
        return runtime;
    }
    panic!("called `current` from the outside of a runtime context.")
}

pub fn block_on<F: Future>(f: F) -> io::Result<F::Output> {
    Runtime::new()?.block_on(f)
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
    config: Config,
    driver: Driver,
    executor: Executor,
}

impl Runtime {
    pub fn new() -> io::Result<Runtime> {
        let config = Config::default();
        let driver = Driver::new(config.clone())?;
        let executor = Executor::new();
        let inner = Inner {
            config,
            driver,
            executor,
        };
        let runtime = Runtime(Rc::new(inner));
        Ok(runtime)
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
    /// ```
    /// # fn main() -> std::io::Result<()> {
    /// use osiris::runtime::Runtime;
    ///
    /// // Create the runtime
    /// let rt  = Runtime::new()?;
    ///
    /// // Execute the future, blocking the current thread until completion
    /// rt.block_on(async {
    ///     println!("hello");
    /// });
    /// # Ok(())}
    /// ```
    ///
    /// [handle]: fn@Handle::block_on
    #[track_caller]
    pub fn block_on<F>(&self, mut future: F) -> io::Result<F::Output>
    where
        F: Future,
    {
        assert!(
            Runtime::current().is_none(),
            "called `block_on` from the inside of another osiris runtime."
        );

        let _h = self.enter();

        let Inner {
            executor,
            config,
            driver,
        } = &*self.0;

        pin!(future);
        let mut handle = unsafe { executor.block_on_spawn(future) };
        let waker = waker(0);
        let ref mut cx = Context::from_waker(&waker);
        waker.wake_by_ref();
        loop {
            executor.poll(config.event_interval);

            if executor.is_woken() {
                // driver.submit_yield()?;
            } else {
                // driver.submit_wait()?;
            }
            let handle = unsafe { Pin::new_unchecked(&mut handle) };
            if let Poll::Ready(out) = handle.poll(cx) {
                return Ok(out);
            }
        }
    }

    pub fn enter(&self) -> impl Drop + '_ {
        struct Enter<'a>(Option<Runtime>, &'a ());
        impl<'a> Drop for Enter<'a> {
            fn drop(&mut self) {
                RUNTIME.with(|cell| cell.replace(self.0.take()));
            }
        }
        let new_rt = Some(self.clone());
        let rt = RUNTIME.with(|cell| cell.replace(new_rt));
        Enter(rt, &())
    }
}
