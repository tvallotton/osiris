use self::shared_task::SharedTask;
use crate::runtime::Runtime;
use std::any::Any;
use std::future::Future;
use std::task::{Context, Poll, Waker};

pub use completion::complete;
pub use fns::{id, spawn};
pub use join_handle::JoinHandle;
pub(crate) use waker::waker;
pub use yield_now::yield_now;

mod completion;
mod fns;
mod join_handle;
mod raw_task;
mod shared_task;
mod task_repr;
mod waker;
mod yield_now;

#[derive(Clone)]
pub(crate) struct Task {
    pub(crate) shared: SharedTask,
    id: usize,
    detached: bool,
}

impl Task {
    pub(crate) fn new<F: Future + 'static>(id: usize, fut: F, rt: Runtime) -> Task {
        Task {
            shared: SharedTask::new(fut, rt),
            id,

            detached: false,
        }
    }

    pub(crate) fn id(&self) -> usize {
        self.id
    }

    pub(crate) fn poll(&self, cx: &mut Context) -> Poll<()> {
        self.shared.task().poll(cx)
    }
    /// Aborts the task. For the moment, it is not supported for a task
    /// to abort itself.
    pub(crate) fn abort(&self) {
        self.shared.task().abort();
    }
    /// Sets the panic payload for the task in case it panicked while being polled
    pub(crate) fn panic(&self, payload: Box<dyn Any + Send>) {
        self.shared.task().panic(payload);
    }
    pub(crate) fn waker(self) -> Waker {
        waker(self.shared)
    }
}
