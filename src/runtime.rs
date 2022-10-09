use std::cell::RefCell;
use std::future::Future;
use std::io;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use self::config::Config;
use crate::pin;
use crate::runtime::waker::{main_waker, waker};
use crate::task::JoinHandle;
use driver::Driver;
use executor::Executor;

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
    ///
    /// After `block_on` returns any pending spawned tasks will remain in the
    ///
    /// Any spawned tasks will be suspended after `block_on` returns. Calling
    /// `block_on` again will resume previously spawned tasks.
    ///
    /// # Panics
    ///
    /// This function panics if the blocked on future panics.
    /// Panics on children tasks are catched.
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> std::io::Result<()> {
    /// use osiris::runtime::Runtime;
    /// use osiris::task::yield_now;
    ///
    /// // Create the runtime
    /// let rt  = Runtime::new()?;
    ///
    /// // Execute the future, blocking the current thread until completion
    /// rt.block_on(async {
    ///     yield_now().await;
    ///     println!("hello");
    /// });
    /// # Ok(())}
    /// ```
    ///
    /// [handle]: fn@Handle::block_on
    pub fn block_on<F>(&self, future: F) -> io::Result<F::Output>
    where
        F: Future,
    {
        pin!(future);
        loop {
            let event_loop = AssertUnwindSafe(|| self.event_loop(future));
            match catch_unwind(event_loop) {
                Ok(result) => return result,
                Err(error) => {
                    let queue = self.0.executor.tasks.borrow();
                    if queue.contains_key(&0) {
                        continue;
                    }
                    resume_unwind(error);
                }
            }
        }
    }
    /// This is the main loop
    fn event_loop<F: Future>(&self, future: Pin<&mut F>) -> io::Result<F::Output> {
        assert!(
            Runtime::current().is_none(),
            "called `block_on` from the inside of another osiris runtime."
        );
        // we enter the runtime context so functions like `spawn` are
        // available.
        let _h = self.enter();

        let Inner {
            executor,
            config,
            driver,
        } = &*self.0;

        // # Safety
        // This operation is safe because the task will not outlive the function scope.
        // This is also true for the case of a panic. If the main task panicked on `poll`,
        // it will not have been returned to the task queue, and will have been dropped in
        // the call to `Executor::poll`.
        let mut handle = unsafe { executor.block_on_spawn(future) };

        // we make sure the main task is awake so it gets executed.
        waker(0).wake();
        // we also make sure the waker for the JoinHandle gets registered
        // by polling the JoinHandle before polling the main task.
        executor.main_awoken.set(true);
        let main_waker = main_waker();
        let ref mut cx = Context::from_waker(&main_waker);

        loop {
            let handle = Pin::new(&mut handle);

            if executor.main_awoken.get() {
                if let Poll::Ready(out) = handle.poll(cx) {
                    return Ok::<_, io::Error>(out);
                }
            }

            executor.poll(config.event_interval);

            if executor.is_woken() {
                driver.submit_yield()?;
            } else {
                driver.submit_wait()?;
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

    pub fn clear(&mut self) {
        todo!()
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        todo!()
    }
}
