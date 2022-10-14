use crate::runtime::current_unwrap;
pub use join_handle::JoinHandle;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
pub use yield_now::yield_now;

use self::raw_task::RawTask;
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

pub(crate) trait Task {
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
/// This a wrapper for pinned tasks that will abort them on drop.
/// which is used to make sure the runtime aborts tasks that panic. 
pub(crate) struct AbortOnDrop(Pin<Rc<dyn Task>>);

impl AbortOnDrop {
    pub(crate) fn new<F: Future + 'static>(task_id: usize, fut: F) -> AbortOnDrop {
        AbortOnDrop(Rc::pin(RawTask::new(task_id, fut)))
    }

    pub(crate) fn task(&self) -> Pin<Rc<dyn Task>> {
        self.0.clone()
    }
}
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.as_ref().abort_in_place();
    }
}
