use super::Task;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

pub struct JoinHandle<T> {
    task: Pin<Rc<dyn Task>>,
    _t: PhantomData<T>,
}

impl<T> JoinHandle<T> {
    pub(crate) fn new(task: Pin<Rc<dyn Task>>) -> JoinHandle<T> {
        JoinHandle {
            task,
            _t: PhantomData::default(),
        }
    }
}

impl<T: 'static> Future for JoinHandle<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut output: Poll<T> = Poll::Pending;
        self.task.as_ref().poll_join(cx, &mut output);
        output
    }
}
