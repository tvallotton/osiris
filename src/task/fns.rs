use super::JoinHandle;
use crate::runtime::current_unwrap;
use std::future::Future;

/// Spawns a new asynchronous task, returning a
/// [`JoinHandle`](JoinHandle) for it. When the [`JoinHandle`](JoinHandle)
/// gets dropped the spawned task will get cancelled.
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
#[must_use = "This task is immediatly cancelled after spawn. osiris tasks are cancelled on drop, you may want to use `detach()`."]
pub fn spawn<F>(future: F) -> JoinHandle<<F as Future>::Output>
where
    F: Future + 'static,
{
    current_unwrap("spawn").spawn(future)
}

/// Spawns a new asynchronous task, returning a
/// [`JoinHandle`](JoinHandle) for it. Unlike `spawn`, detached tasks
/// will not be cancelled when the [`JoinHandle`](JoinHandle) gets dropped.
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
#[track_caller]
pub fn detach<F>(future: F) -> JoinHandle<<F as Future>::Output>
where
    F: Future + 'static,
{
    current_unwrap("detach").detach(future)
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
pub fn id() -> u64 {
    crate::runtime::TASK_ID
        .with(Clone::clone)
        .get()
        .expect("called `task::id()` from the outside of an osiris task.")
}
