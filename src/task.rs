use self::raw_task::RawTask;
use crate::runtime::current_unwrap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

pub use completion::complete;
pub use join_handle::JoinHandle;
pub use yield_now::yield_now;

mod completion;
mod join_handle;
mod raw_task;
mod yield_now;

/// Spawns a new asynchronous task, returning a
/// [`JoinHandle`](JoinHandle) for it.
///
/// Spawning a task enables the task to execute concurrently to other tasks. The
/// spawned task will execute on the current thread.
///
/// There is no guarantee that a spawned task will execute to completion.
/// When a runtime is shutdown, all outstanding tasks are dropped,
/// regardless of the lifecycle of that task.
///
/// This function must be called from the context of an osiris runtime. Tasks running on
/// a osiris runtime are always inside its context, but you can also enter the context
/// using the [`Runtime::enter`](crate::runtime::Runtime::enter()) method.
///
/// # Panics
///
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
/// is guaranteed to be a unique identifier for the current task.
/// They may be reused after a task is driven to completion. Task ids
/// are not guaranteed to be unique across runtimes. This means that if
/// multiple osiris runtimes are spinned up in separate threads, their task
/// identifiers will not be unique.
///
/// # Example
///
/// ```rust
/// use osiris::task::task_id;
/// use osiris::spawn;
///
/// # osiris::block_on(async {
/// println!("main task, id: {}", task_id());
/// spawn(async {
///     println!("spawned task, id: {}", task_id());
/// }).await;
/// # });
/// ```
///
/// # Panics
/// Panics if called from the **outside** of an osiris async task.
#[track_caller]
pub fn task_id() -> usize {
    crate::runtime::TASK_ID
        .with(|task_id| task_id.clone())
        .get()
        .expect("called `task_id()` from the outside of a task context.")
}

/// Aborts from the current task abnormally.
/// Note that attempting to join a cancelled task will panic.
///
pub async fn abort() -> ! {
    current_unwrap("task::exit()")
        .0
        .executor
        .aborted
        .borrow_mut()
        .push_back(task_id());
    std::future::pending().await
}

pub(crate) trait Task {
    fn wake_join(&self);
    fn abort(self: Pin<&Self>);
    fn abort_in_place(self: Pin<&Self>);
    fn poll(self: Pin<&Self>, cx: &mut Context) -> Poll<()>;
    unsafe fn poll_join(self: Pin<&Self>, cx: &mut Context, ptr: *mut ());
}

impl dyn Task {
    pub(crate) fn new<F: Future + 'static>(task_id: usize, fut: F) -> Pin<Rc<dyn Task>> {
        Rc::pin(RawTask::new(task_id, fut))
    }
}
