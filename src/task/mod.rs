//! Asynchronous coroutines.
//!
//! ## What are Tasks?
//!
//! A _task_ is a light weight, non-blocking unit of execution. A task is similar
//! to an OS thread, but rather than being managed by the OS scheduler, they are
//! managed by the [Osiris runtime][crate::runtime::Runtime]. Another name for
//! this general pattern is [green threads]. If you are familiar with [Go's goroutines],
//! [Kotlin's coroutines], or [Erlang's processes], you can think of Osiris's tasks as
//! something similar.
//!
//! Key points about tasks include:
//!
//! * Tasks are **light weight**. Because tasks are scheduled by the Tokio
//!   runtime rather than the operating system, creating new tasks or switching
//!   between tasks does not require a context switch and has fairly low
//!   overhead. Creating, running, and destroying large numbers of tasks is
//!   quite cheap, especially compared to OS threads.
//!
//! * Tasks are scheduled **cooperatively**. Most operating systems implement
//!   _preemptive multitasking_. This is a scheduling technique where the
//!   operating system allows each thread to run for a period of time, and then
//!   _preempts_ it, temporarily pausing that thread and switching to another.
//!   Tasks, on the other hand, implement _cooperative multitasking_. In
//!   cooperative multitasking, a task is allowed to run until it _yields_,
//!   indicating to the runtime's scheduler that it cannot currently
//!   continue executing. When a task yields, the runtime switches to
//!   executing the next task.
//!
//! * Tasks are **non-blocking**. Typically, when an OS thread performs I/O or
//!   must synchronize with another thread, it _blocks_, allowing the OS to
//!   schedule another thread. When a task cannot continue executing, it must
//!   yield instead, allowing the runtime to schedule another task. Tasks
//!   should generally not perform system calls or other operations that could
//!   block a thread, as this would prevent other tasks running on the same
//!   thread from executing as well. Instead, this module provides APIs for
//!   running blocking operations in an asynchronous context.
//!
//! [rt]: crate::runtime
//! [green threads]: https://en.wikipedia.org/wiki/Green_threads
//! [Go's goroutines]: https://tour.golang.org/concurrency/1
//! [Kotlin's coroutines]: https://kotlinlang.org/docs/reference/coroutines-overview.html
//! [Erlang's processes]: http://erlang.org/doc/getting_started/conc_prog.html#processes
//!
//! ## Working with Tasks
//!
//! This module provides the following APIs for working with tasks:
//!
//! ### Spawning
//!
//! Perhaps the most important function in this module is [`task::spawn`]. This
//! function can be thought of as an async equivalent to the standard library's
//! [`thread::spawn`][`std::thread::spawn`]. It takes an `async` block or other
//! [future], and creates a new task to run that work concurrently. Like [`std::thread::spawn`],
//! `task::spawn` returns a [`JoinHandle`] struct. A `JoinHandle` is itself a future which
//! may be used to await the output of the spawned task. For example:
//!
//! ```no_run
//! # async {
//! use osiris::task;
//!
//! let join = task::spawn(async {
//!     // ...
//!     "hello world!"
//! });
//!
//! // ...
//!
//! // Await the result of the spawned task.
//! let result = join.await;
//! assert_eq!(result, "hello world!");
//! # };
//! ```
//!
//! #### Detached tasks
//! Osiris follows structured concurrency which discrouages orphan tasks. For this reason,
//! when a [`JoinHandle`] gets dropped, osiris will automatically cancel that task to prevent
//! it from dangling and outliving its parent.
//!
//! ```no_run
//! # use osiris::task;
//! let join = task::spawn(async {
//!     println!("this will never get printed,");
//! });
//! // cancel the task.
//! drop(join);
//! ```
//!
//! Sometimes we need tasks to be detached from their parents. We have the `task::detach` method for
//! this purpose:
//! ```
//! # async {
//! # use osiris::task;
//! task::detach(async {
//!     // some code
//! });
//! # };
//! ```
//!
//! Unlike tokio and other runtimes, if the task panics, the join handle will propagate the panic to the parent.
//! The panic will be propagated both on aborts and on joins. In order to handle the panic gracefully from the parent
//! the `catch_unwind` method can be used.
//!
//! ```no_run
//! use osiris::task;
//!
//! # async {
//! let join = task::spawn(async {
//!     panic!("something bad happened!")
//! });
//!
//! // The returned result indicates that the task failed.
//! assert!(join.catch_unwind().await.is_err());
//! # };
//! ```
//!
//! `spawn`, `JoinHandle`, and `JoinError` are present when the "rt"
//! feature flag is enabled.
//!
//! [`task::spawn`]: crate::task::spawn()
//! [future]: std::future::Future
//! [`std::thread::spawn`]: std::thread::spawn
//! [`JoinHandle`]: crate::task::JoinHandle
//! [thread_join]: std::thread::JoinHandle
//! [`JoinError`]: crate::task::JoinError
//!
//! ### Blocking and Yielding
//!
//! As we discussed above, code running in asynchronous tasks should not perform
//! operations that can block. A blocking operation performed in a task running
//! on a thread that is also running other tasks would block the entire thread,
//! preventing other tasks from running.
//!
//! Instead, Osiris provides two APIs for running blocking operations in an
//! asynchronous context: [`task::spawn_blocking`] and [`task::block_in_place`].
//!
//! Be aware that if you call a non-async method from async code, that non-async
//! method is still inside the asynchronous context, so you should also avoid
//! blocking operations there. This includes destructors of objects destroyed in
//! async code.
//!
//! #### spawn_blocking
//!
//! The `task::spawn_blocking` function is similar to the `task::spawn` function
//! discussed in the previous section, but rather than spawning an
//! _non-blocking_ future on the Tokio runtime, it instead spawns a
//! _blocking_ function on a dedicated thread pool for blocking tasks. For
//! example:
//!
//! ```no_run
//! use osiris::task;
//!
//! # async fn docs() {
//! task::spawn_blocking(|| {
//!     // do some compute-heavy work or call synchronous code
//! });
//! # }
//! ```
//!
//! Just like `task::spawn`, `task::spawn_blocking` returns a `JoinHandle`
//! which we can use to await the result of the blocking operation:
//!
//! ```no_run
//! # use osiris::task;
//! # async {
//! let join = task::spawn_blocking(|| {
//!     // do some compute-heavy work or call synchronous code
//!     "blocking completed"
//! });
//!
//! let result = join.await;
//! assert_eq!(result, "blocking completed");
//! # };
//! ```
//!
//! #### yield_now
//!
//! In addition, this module provides a [`task::yield_now`] async function
//! that is analogous to the standard library's [`thread::yield_now`]. Calling
//! and `await`ing this function will cause the current task to yield to the
//! Osiris runtime's scheduler, allowing other tasks to be
//! scheduled. Eventually, the yielding task will be polled again, allowing it
//! to execute. For example:
//!
//! ```no_run
//! use osiris::task;
//!
//! # async {
//! task::spawn(async {
//!     // ...
//!     println!("spawned task done!")
//! });
//!
//! // Yield, allowing the newly-spawned task to execute first.
//! task::yield_now().await;
//! println!("main task done!");
//! # };
//!
//! ```
//!
//! [`task::spawn_blocking`]: crate::task::spawn_blocking
//! [`task::block_in_place`]: crate::task::block_in_place
//! [rt-multi-thread]: ../runtime/index.html#threaded-scheduler
//! [`task::yield_now`]: crate::task::yield_now()
//! [`thread::yield_now`]: std::thread::yield_now
//! [`task::unconstrained`]: crate::task::unconstrained()
//! [`poll`]: method@std::future::Future::poll

use self::shared_task::SharedTask;

use std::any::Any;

use std::task::{Context, Waker};

pub use fns::{detach, id, spawn};
pub use join_handle::JoinHandle;
pub use spawn_blocking::spawn_blocking;
pub(crate) use waker::waker;
pub use yield_now::yield_now;

mod fns;
mod join_handle;
mod meta;
mod raw_task;
mod shared_task;
mod spawn_blocking;
mod task_repr;
mod waker;
mod yield_now;

pub(crate) type Task = SharedTask;

impl Task {
    #[inline]
    pub(crate) fn id(&self) -> u64 {
        self.meta().id
    }

    pub(crate) fn poll(&self, cx: &mut Context) {
        self.task().poll(cx);
    }
    /// Aborts the task. For the moment, it is not supported for a task
    /// to abort itself.
    pub(crate) fn abort(&self) {
        if !self.meta().ignore_abort {
            self.task().abort();
        }
    }
    /// Sets the panic payload for the task in case it panicked while being polled
    pub(crate) fn panic(&self, payload: Box<dyn Any + Send>) {
        self.task().panic(payload);
    }
    pub(crate) fn waker(self) -> Waker {
        waker(self)
    }
}
