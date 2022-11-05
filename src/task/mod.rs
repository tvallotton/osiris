use crate::runtime::current_unwrap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

pub use completion::complete;
pub use join_handle::JoinHandle;
pub use yield_now::yield_now;

use self::task_repr::TaskRepr;

mod completion;
mod join_handle;
mod task_repr;
mod yield_now;

/// Spawns a new asynchronous task, returning a
/// [`JoinHandle`](JoinHandle) for it.
///
/// Spawning a task enables the future to be executed concurrently with respect to other tasks.
/// The spawned task will execute on the current thread.
///
/// There is no guarantee that a spawned task will execute to completion.
/// When a runtime is shutdown, all outstanding tasks are dropped,
/// regardless of the lifecycle of that task.
///
/// This function must be called from the context of an osiris runtime. Tasks running on
/// a osiris runtime are always inside its context, but you can also enter the context
/// using the [`Runtime::enter()`](crate::runtime::Runtime::enter) method.
///
/// # Panics
/// Panics if called from **outside** of an osiris runtime.
///
#[track_caller]
pub fn spawn<F>(future: F) -> JoinHandle<<F as Future>::Output>
where
    F: Future + 'static,
{
    current_unwrap("spawn").spawn(future)
}
/// Returns the task id for the currently running task. The task id
/// is guaranteed to be a unique identifier. They may be reused after
/// a task is driven to completion. Task ids are not guaranteed to be
/// unique across runtimes. This means that if multiple osiris runtimes
/// are spinned up in separate threads, their task identifiers will not
/// be unique.
///
/// # Example
///
/// ```rust
/// use osiris::task;
/// use osiris::spawn;
///
/// # osiris::block_on(async {
/// println!("main task, id: {}", task::id());
/// spawn(async {
///     println!("spawned task, id: {}", task::id());
/// }).await;
/// # });
/// ```
///
/// # Panics
/// Panics if called from the **outside** of an osiris async task.
#[track_caller]
#[must_use]
pub fn id() -> usize {
    crate::runtime::TASK_ID
        .with(Clone::clone)
        .get()
        .expect("called `task_id()` from the outside of a task context.")
}

/// Aborts from the current task abnormally.
/// Note that attempting to join a cancelled task will panic.
/// As a result, calling this function may cause the parent task to panic.
pub async fn abort() -> ! {
    current_unwrap("task::exit()")
        .executor
        .aborted
        .borrow_mut()
        .push_back(id());
    std::future::pending().await
}
#[derive(Clone)]
pub(crate) struct Task {
    raw: Pin<Rc<dyn RawTask>>,
    id: usize,
    detached: bool,
}

impl Task {
    pub(crate) fn new<F: Future + 'static>(id: usize, fut: F) -> Task {
        Task {
            raw: Rc::pin(TaskRepr::new(fut)),
            id,
            detached: false,
        }
    }

    pub(crate) fn id(&self) -> usize {
        self.id
    }

    pub(crate) fn poll(&self, cx: &mut Context) -> Poll<()> {
        self.raw.as_ref().poll(cx)
    }

    /// Aborts a task immediately. Beware not to call this from
    /// the inside a poll function, which might trigger a panic
    /// if a task attempts to abort itself.
    pub(crate) fn abort_in_place(&self) {
        self.raw.as_ref().abort_in_place();
    }

    /// Schedules the task to be aborted by the end of the event cycle.
    fn abort(&self) {
        current_unwrap("JoinHandle::abort")
            .executor
            .aborted
            .borrow_mut()
            .push_back(self.id);
    }
}

pub(crate) trait RawTask {
    fn wake_join(&self);
    fn abort_in_place(self: Pin<&Self>);
    fn poll(self: Pin<&Self>, cx: &mut Context) -> Poll<()>;
    unsafe fn poll_join(self: Pin<&Self>, cx: &mut Context, ptr: *mut ());
}

impl Drop for Task {
    fn drop(&mut self) {
        if self.detached {}
    }
}
