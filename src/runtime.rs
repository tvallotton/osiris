use std::cell::{Cell, RefCell};
use std::future::Future;
use std::io;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use crate::driver::Driver;
use crate::runtime::waker::{main_waker, waker};
use crate::task::JoinHandle;
pub use config::{Config, Mode};
use executor::Executor;

mod config;

mod executor;
mod unique_queue;
pub(crate) mod waker;

thread_local! {
    pub(crate) static RUNTIME: RefCell<Option<Runtime>>= RefCell::new(None);
}

thread_local! {
    pub(crate) static TASK_ID: Cell<Option<usize>> = Cell::new(None);
}

/// Returns a handle to the currently running [`Runtime`].
/// # Panics
/// This will panic if called outside the context of a osiris runtime.
/// It is ok to call this function from a spawned task or from a [blocked on](block_on) future.
#[track_caller]
pub fn current() -> Option<Runtime> {
    RUNTIME.with(|cell| cell.borrow().clone())
}

pub fn block_on<F: Future>(f: F) -> io::Result<F::Output> {
    Runtime::new()?.block_on(f)
}

#[track_caller]
pub(crate) fn current_unwrap(fun: &str) -> Runtime {
    if let Some(rt) = current() {
        return rt;
    }
    panic!("called `{fun}` from the outside of a runtime context.")
}

/// The osiris local runtime.
/// For the moment it cannot be customized.
#[derive(Clone)]
pub struct Runtime(pub(crate) Rc<Inner>);

pub(crate) struct Inner {
    pub config: Config,
    pub driver: Rc<Driver>,
    pub executor: Executor,
}

impl Runtime {
    /// Creates a new osiris runtime with the default configuration values.
    /// For more information on the default configuration, check out the [`Config`].
    /// struct.
    pub fn new() -> io::Result<Runtime> {
        let config = Config::default();
        let driver = Rc::new(Driver::new(config.clone())?);
        let executor = Executor::new();
        let inner = Inner {
            config,
            driver,
            executor,
        };
        let runtime = Runtime(Rc::new(inner));
        Ok(runtime)
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
    pub fn block_on<F>(&self, mut future: F) -> io::Result<F::Output>
    where
        F: Future,
    {
        loop {
            // SAFETY: The future is never moved
            let future = unsafe { Pin::new_unchecked(&mut future) };

            let event_loop = AssertUnwindSafe(move || self.event_loop(future));
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
            current().is_none(),
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

        // # Safety:
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
        let cx = &mut Context::from_waker(&main_waker);

        TASK_ID.with(|task_id| {
            loop {
                let handle = Pin::new(&mut handle);

                if executor.main_awoken.get() {
                    if let Poll::Ready(out) = handle.poll(cx) {
                        return Ok::<_, io::Error>(out);
                    }
                }

                executor.poll(config.event_interval, task_id);
                executor.remove_aborted();
                if executor.is_woken() {
                    driver.submit_and_yield()?;
                } else {
                    driver.submit_and_wait()?;
                }
                // driver.poll();
            }
        })
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
    fn drop(&mut self) {}
}
