use super::Task;
use crate::runtime::waker::waker;

use std::{
    any::Any,
    cell::UnsafeCell,
    future::Future,
    hint::unreachable_unchecked,
    mem::replace,
    pin::Pin,
    task::{Context, Poll, Waker},
};

pub(crate) struct RawTask<F: Future> {
    // Tasks are shared mutable state
    // so we need to enclose its contents in a cell.
    // althought this is an UnsafeCell its contents are pinned.
    cell: UnsafeCell<Inner<F>>,
}

pub(crate) struct Inner<F: Future> {
    pub join_waker: Option<Waker>,
    pub task_id: usize,
    pub payload: Payload<F>,
}

pub(crate) enum Payload<F: Future> {
    Taken,
    Aborted,
    Pending { fut: F },
    Ready { output: F::Output },
}

impl<F: Future> RawTask<F> {
    pub fn new(task_id: usize, fut: F) -> Self {
        RawTask {
            cell: UnsafeCell::new(Inner {
                join_waker: None,
                task_id,
                payload: Payload::Pending { fut },
            }),
        }
    }
}

impl<F: Future> Inner<F> {
    fn wake_join(&mut self) {
        if let Some(waker) = &self.join_waker {
            waker.wake_by_ref();
        }
    }
    fn insert_waker(&mut self, cx: &mut Context) {
        let _ = self.join_waker.insert(cx.waker().clone());
    }
}

impl<F: Future> Task for RawTask<F>
where
    F::Output: 'static,
{
    fn poll(self: Pin<&Self>, cx: &mut Context) -> Poll<()> {
        // SAFETY: this is ok because the reference does not outlive the function.
        //         thus, there cannot be two references to this task.
        let task = unsafe { &mut *self.cell.get() };

        if let Payload::Pending { fut } = &mut task.payload {
            // SAFETY: this is ok because fut: &mut F is never moved,
            //         so we can project the pin.
            let fut = unsafe { Pin::new_unchecked(fut) };

            if let Poll::Ready(output) = fut.poll(cx) {
                // this is ok because the future gets dropped.
                task.payload = Payload::Ready { output };
                // let's wake the joining task.
                task.wake_join();
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        } else {
            // we return ready so the task can be removed from the queue.
            Poll::Ready(())
        }
    }
    #[track_caller]
    fn poll_join(self: Pin<&Self>, cx: &mut Context, out: &mut dyn Any) {
        // SAFETY: this is ok because the reference does not outlive the function.
        //         thus, there cannot be two references to this task.
        let task = unsafe { &mut *self.cell.get() };
        task.insert_waker(cx);
        if !matches!(task.payload, Payload::Pending { .. }) {
            let payload = replace(&mut task.payload, Payload::Taken);

            match payload {
                Payload::Ready { output } => {
                    let out: &mut Poll<F::Output> = out.downcast_mut().unwrap();
                    *out = Poll::Ready(output);
                }
                Payload::Taken => {
                    panic!("polled a JoinHandle future after returning Poll::Ready(..).");
                }
                Payload::Aborted => {
                    panic!("attempted to join a task that has been aborted.")
                }
                // SAFETY: we already checked for this case
                Payload::Pending { .. } => unsafe { unreachable_unchecked() },
            }
        }
    }

    fn abort(self: Pin<&Self>) {
        // SAFETY: this is ok because the reference does not outlive the current scope.
        //         thus, there cannot be two references to this task.
        let task = unsafe { &mut *self.cell.get() };
        task.payload = Payload::Aborted;
        waker(task.task_id).wake();
    }
}
