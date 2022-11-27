use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use super::raw_task::RawTask;
use super::task_repr::TaskRepr;
use crate::runtime::Runtime;

#[derive(Clone)]
pub(crate) struct Task {
    pub(crate) raw: Pin<Rc<dyn RawTask>>,
    rt: Runtime,
    id: usize,
    detached: bool,
}

impl Task {
    pub(crate) fn new<F: Future + 'static>(id: usize, fut: F, rt: Runtime) -> Task {
        Task {
            raw: Rc::pin(TaskRepr::new(fut)),
            id,
            rt,
            detached: false,
        }
    }

    pub(crate) fn id(&self) -> usize {
        self.id
    }

    pub(crate) fn poll(&self, cx: &mut Context) -> Poll<()> {
        self.raw.as_ref().poll(cx)
    }
    /// Aborts the task calling its destructor.
    pub(crate) fn abort(&self) {
        self.raw.as_ref().abort();
        self.rt.executor.tasks.borrow_mut().remove(&self.id);
    }

    pub(crate) fn panicked(&self, payload: Box<dyn Any + Send>) {
        self.raw.as_ref().panicked(payload);
    }
}
