use crate::runtime::current_unwrap;
use std::future::Future;

pub use completion::complete;
pub use join_handle::JoinHandle;
pub(crate) use shared_task::Task;
pub use yield_now::yield_now;

mod completion;
mod join_handle;
mod raw_task;
mod shared_task;
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
