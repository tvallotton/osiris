use super::Task;
use crate::runtime::waker::waker;
use pin_project_lite::pin_project;
use std::{
    any::Any,
    future::{Future, Pending, Ready},
    hint::unreachable_unchecked,
    mem::replace,
    pin::Pin,
    task::{Context, Poll, Waker},
};

pub(crate) struct RawTask<F: Future> {
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
    fn wake_join(&self) {
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
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        // SAFETY: F is never moved.
        let task = unsafe { Pin::get_unchecked_mut(self) };
        // this is ok because fut is a &mut F.
        if let Payload::Pending { fut } = &mut task.payload {
            // We can pin it back because we never move it.
            let fut = unsafe { Pin::new_unchecked(fut) };
            if let Poll::Ready(output) = fut.poll(cx) {
                // this is ok because the future gets dropped.
                task.payload = Payload::Ready { output };
                task.wake_join();
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        } else {
            Poll::Ready(())
        }
    }
    #[track_caller]
    fn poll_join(self: Pin<&mut Self>, cx: &mut Context, out: &mut dyn Any) {
        // SAFETY: F is never moved.
        let task = unsafe { Pin::get_unchecked_mut(self) };
        task.insert_waker(cx);

        if !matches!(task.payload, Payload::Pending { .. }) {
            let payload = replace(&mut task.payload, Payload::Taken);

            match payload {
                Payload::Ready { output } => {
                    let out: &mut Option<F::Output> = out.downcast_mut().unwrap();
                    let _ = out.insert(output);
                }
                Payload::Taken => {
                    panic!("polled a JoinHandle future after returning Poll::Ready(..).");
                }
                Payload::Aborted => {
                    panic!("attempted to join a task that has been aborted.")
                }
                Payload::Pending { .. } => unsafe { unreachable_unchecked() },
            }
        }
    }

    fn wake(self: Pin<&mut Self>) {
        waker(self.task_id).wake();
    }
    
    fn abort(self: Pin<&mut Self>) {
        // SAFETY: F is never moved.
        let task = unsafe { Pin::get_unchecked_mut(self) };
        task.payload = Payload::Aborted;
        // we wake it to make sure it gets destroyed in the next tick
    }
}
