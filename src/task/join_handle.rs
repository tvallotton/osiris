use super::Task;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

/// A handle to the spawned task. By default the task will be cancelled
/// when the join handle gets dropped. In order to detach on drop the
/// (JoinHandle::detach)[`.detach()`] method should be called.
///
/// # Panics
/// Awating a task will panic if the awaited task panicked.
pub struct JoinHandle<T> {
    task: Pin<Rc<dyn Task>>,
    detached: bool,
    _t: PhantomData<T>,
}

impl<T> Unpin for JoinHandle<T> {}

impl<T> JoinHandle<T> {
    /// Detaches the task from the join handle, meaning it will not
    /// get cancelled when the handle gets dropped.
    #[inline]
    pub fn detach(&mut self) {
        self.detached = true;
    }
}

impl<T> JoinHandle<T> {
    pub(crate) fn new(task: Pin<Rc<dyn Task>>) -> JoinHandle<T> {
        JoinHandle {
            task,
            detached: false,
            _t: PhantomData::default(),
        }
    }
    /// This function will schedule the task to be aborted in the next event loop.  
    /// The task is not guaranteed to be cancelled immediately. It may still be possible
    /// for the task to be finished before it gets aborted.
    pub fn abort(self) {
        self.task.as_ref().abort();
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut output: Poll<T> = Poll::Pending;
        let ptr = &mut output as *mut _ as *mut ();
        // SAFETY:
        // The output type is the same as the JoinHandle since a
        // JoinHandle<T> cannot be constructed from a task of a
        // type different from T.
        unsafe { self.task.as_ref().poll_join(cx, ptr) };
        output
    }
}

impl<T> Drop for JoinHandle<T> {
    fn drop(&mut self) {
        if !self.detached {
            self.task.as_ref().abort();
        }
    }
}
