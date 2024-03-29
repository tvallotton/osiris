use std::any::Any;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::utils::futures::catch_unwind;

use super::Task;

/// A handle to the spawned task. By default the task will be cancelled
/// when the join handle gets dropped. In order to detach on drop the
/// [`detach()`](crate::task::detach) method should be called.
///
/// # Panics
/// Awating a task will panic if the awaited task panicked.
/// Dropping a task will atomatically cancel the spawned task.
pub struct JoinHandle<T> {
    task: Task,
    detached: bool,
    _t: PhantomData<T>,
}

impl<T> Unpin for JoinHandle<T> {}

impl<T> JoinHandle<T> {
    /// Detaches the task from the join handle, meaning it will not
    /// get cancelled when the handle gets dropped. This also implies
    /// the task will not propagate panics to the parent task on drop.
    ///
    /// Detached tasks can be aborted with the [`JoinHandle::abort`] method.
    #[inline]
    pub(crate) fn detach(&mut self) {
        self.detached = true;
    }

    #[must_use]
    pub fn id(&self) -> u64 {
        self.task.id()
    }

    /// Aborts the task and runs the spawned future's destructor.
    /// Unlike, other runtimes, osiris tasks are guaranteed to be cancelled immediately.
    /// This is primarily intended for aborting detached tasks, since normal tasks can be
    /// aborted by dropping them. Note that the cancelled task may spawn other tasks to
    /// deal with pending io events.
    ///
    /// # Panics
    /// If the cancelled task panicked, or if a task attempts to cancel itself.
    pub fn abort(mut self) {
        self.detached = false;
    }

    /// Joins the task catching any propagated panics.
    ///
    /// # Errors
    /// Returns an error if the task panicked, with the panic
    /// payload represented as a `Box<dyn Any + Send>`.
    pub async fn catch_unwind(self) -> Result<T, Box<dyn Any + Send + 'static>> {
        catch_unwind(self).await
    }
}

impl<T> JoinHandle<T> {
    /// Safety
    /// The caller must make sure that the output of the task is the same as the output
    /// of the [`JoinHandle`].
    pub(crate) unsafe fn new(task: Task) -> JoinHandle<T> {
        JoinHandle {
            task,
            detached: false,
            _t: PhantomData,
        }
    }
}

impl<T> Drop for JoinHandle<T> {
    fn drop(&mut self) {
        if !self.detached {
            self.task.abort();
        }
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut output: Poll<T> = Poll::Pending;
        let ptr = &mut output as *mut _ as *mut ();
        // Safety:
        // The output type is the same as the JoinHandle since a
        // JoinHandle<T> cannot be constructed from a task of a
        // type different from T.
        unsafe { self.task.task().poll_join(cx, ptr) };
        output
    }
}
