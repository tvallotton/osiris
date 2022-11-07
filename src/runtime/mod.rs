use crate::runtime::waker::{main_waker, waker};
use crate::shared_driver::SharedDriver;
use crate::task::JoinHandle;
use executor::Executor;
use std::future::Future;
use std::io;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use std::time::Duration;

pub use config::{Config, Mode};
pub(crate) use globals::{RUNTIME, TASK_ID};

mod config;
mod executor;
mod globals;
mod unique_queue;
pub(crate) mod waker;

/// Returns a handle to the currently running [`Runtime`].
/// # Panics
/// This will panic if called outside the context of a osiris runtime.
/// It is ok to call this function from a spawned task or from a [blocked on](block_on) future.
#[track_caller]
#[must_use]
pub fn current() -> Option<Runtime> {
    RUNTIME.with(|cell| cell.borrow().clone())
}

/// Run a future to completion on the current thread.
/// This function will block the caller until the given future has completed.
///
/// # Errors
/// Errors if the io-ring could not be allocated.
///  
/// # Panics
/// Panics if called from the inside of another osiris runtime.
/// Runtimes cannot be nested.
pub fn block_on<F: Future>(f: F) -> io::Result<F::Output> {
    Runtime::new()?.block_on(f)
}

#[track_caller]
#[inline]
pub(crate) fn current_unwrap(fun: &str) -> Runtime {
    let Some(rt) = current() else {
         panic!("called `{fun}` from the outside of a runtime context.")
    };
    rt
}

/// The osiris local runtime.
#[derive(Clone)]
pub struct Runtime {
    pub(crate) config: Config,
    pub(crate) executor: Rc<Executor>,
    pub(crate) driver: SharedDriver,
}

impl Runtime {
    /// Creates a new osiris runtime with the default configuration values.
    /// For more information on the default configuration, check out the [`Config`].
    /// struct.
    ///
    /// # Errors
    /// This function errors if the io-ring could not be allocated.
    ///
    pub fn new() -> io::Result<Runtime> {
        let config = Config::default();
        let executor = Rc::new(Executor::new());
        let driver = SharedDriver::new(config.clone())?;
        let rt = Runtime {
            config,
            executor,
            driver,
        };
        Ok(rt)
    }
    /// Spawns a new task onto the runtime returning a `JoinHandle` for that task.    
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
    {
        let task = self.executor.spawn(future);
        JoinHandle::new(task)
    }

    /// Runs a future to completion on the osiris runtime. This is the
    /// runtime's entry point.
    ///
    /// This runs the given future on the current thread, blocking until it is
    /// complete, and yielding its resolved result. Any tasks or timers
    /// which the future spawns internally will be executed on the runtime.
    ///
    /// Any spawned tasks will be suspended after `block_on` returns. Calling
    /// `block_on` again will resume previously spawned tasks.
    ///
    /// # Panics
    ///
    /// This function panics if the blocked on future panics.
    /// Panics on children tasks are catched.
    ///
    /// # Errors
    /// This function errors if the io-ring coult not be allocated.
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
    pub fn block_on<F>(&self, mut future: F) -> io::Result<F::Output>
    where
        F: Future,
    {
        let msg = "called `block_on` from the inside of another osiris runtime.";
        assert!(current().is_none(), "{}", msg);

        // we enter the runtime context so functions like `spawn` are
        // available.
        let h = self.enter();

        // SAFETY: The future is never moved
        let future = unsafe { Pin::new_unchecked(&mut future) };

        // # Safety:
        // This operation is safe because the task will not outlive the function scope.
        // This is also true for the case of a panic. If the main task panicked on `poll`,
        // it will not have been returned to the task queue, and will have been dropped in
        // the call to `Executor::poll`.
        let handle = &mut unsafe { self.executor.block_on_spawn(future) };

        // we make sure the main task is woken so it gets executed
        waker(handle.id()).wake();

        // we also make sure the waker for the JoinHandle gets registered
        // by polling the JoinHandle before polling the main task.
        self.executor.main_handle.set(true);

        loop {
            let event_loop = AssertUnwindSafe(|| self.event_loop(handle));
            match catch_unwind(event_loop) {
                Ok(result) => return result,
                Err(error) => {
                    let queue = self.executor.tasks.borrow();
                    // if the main task panicked we resume_unwind.
                    // otherwise we continue we catch it.
                    if !queue.contains_key(&handle.id()) {
                        drop(h);
                        resume_unwind(error);
                    }
                }
            }
        }
    }
    /// This is the main loop
    fn event_loop<T>(&self, handle: &mut JoinHandle<T>) -> io::Result<T> {
        let Runtime {
            executor,
            config,
            driver,
        } = self;

        let handel_waker = main_waker();
        let handle_cx = &mut Context::from_waker(&handel_waker);

        TASK_ID.with(|task_id| loop {
            std::thread::sleep(Duration::from_millis(500));
            // we must poll the JoinHandle before polling the executor
            let handle = Pin::new(&mut *handle);
            if executor.main_handle.get() {
                if let Poll::Ready(out) = handle.poll(handle_cx) {
                    return Ok(out);
                }
                executor.main_handle.set(false);
            }
            executor.poll(config.event_interval, task_id);
            executor.remove_aborted();
            driver.wake_tasks();
            if executor.is_woken() {
                driver.submit_and_yield()?;
            } else {
                driver.submit_and_wait()?;
            }
            driver.wake_tasks();
        })
    }
    /// Enters the runtime context. While the guard is in scope
    /// calls to runtime dependent functions and futures such as
    /// spawn will resolve to the provided runtime.
    #[must_use]
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
