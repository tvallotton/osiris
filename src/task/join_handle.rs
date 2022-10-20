use super::Task;
use std::future::Future;
use std::marker::PhantomData;
use std::mem::transmute;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

pub struct JoinHandle<T> {
    task: Pin<Rc<dyn Task>>,
    _t: PhantomData<T>,
}

impl<T> Unpin for JoinHandle<T> {}

impl<T> JoinHandle<T> {
    pub(crate) fn new(task: Pin<Rc<dyn Task>>) -> JoinHandle<T> {
        JoinHandle {
            task,
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
        unsafe {
            let mut output: Poll<T> = Poll::Pending;
            let ptr = transmute(&mut output);
            self.task.as_ref().poll_join(cx, ptr);
            output
        }
    }
}
