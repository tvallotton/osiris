use std::any::Any;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use crate::runtime::waker::waker;

use super::raw_task::RawTask;
use super::task_repr::TaskRepr;

#[derive(Clone)]
pub(crate) struct Task {
    pub(crate) raw: Pin<Rc<dyn RawTask>>,
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
    /// Aborts the task calling its destructor.
    pub(crate) fn abort(&self) {
        self.raw.as_ref().abort();
        // we wake to make sure the task gets polled.
        waker(self.id).wake();
    }

    pub(crate) fn panicked(&self, payload: Box<dyn Any + Send>) {
        self.raw.as_ref().panicked(payload);
    }
}

impl Debug for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Task")
            .field("id", &self.id)
            .field("detached", &self.detached)
            .field("status", &self.raw.status())
            .finish()
    }
}
