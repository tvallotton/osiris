use self::shared_task::SharedTask;

use std::any::Any;

use std::task::{Context, Waker};

pub use completion::complete;
pub use fns::{id, spawn};
pub use join_handle::JoinHandle;
pub(crate) use waker::waker;
pub use yield_now::yield_now;

mod completion;
mod fns;
mod join_handle;
mod meta;
mod raw_task;
mod shared_task;
mod task_repr;
mod waker;
mod yield_now;

pub(crate) type Task = SharedTask;

impl Task {
    pub(crate) fn id(&self) -> u64 {
        self.meta().id
    }

    pub(crate) fn poll(&self, cx: &mut Context) {
        self.task().poll(cx);
    }
    /// Aborts the task. For the moment, it is not supported for a task
    /// to abort itself.
    pub(crate) fn abort(&self) {
        self.task().abort();
    }
    /// Sets the panic payload for the task in case it panicked while being polled
    pub(crate) fn panic(&self, payload: Box<dyn Any + Send>) {
        self.task().panic(payload);
    }
    pub(crate) fn waker(self) -> Waker {
        waker(self)
    }
}
