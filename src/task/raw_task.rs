use crate::runtime::current_unwrap;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::hint::unreachable_unchecked;
use std::mem::{replace, transmute};
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use super::Task;

pub(crate) struct RawTask<F: Future> {
    /// Even though strictly speaking cells do not pin project,
    /// we will consider the contents of this cell pinned.
    payload: RefCell<Payload<F>>,
    /// we store here the waker for the JoinHandle.
    pub join_waker: Cell<Option<Waker>>,
    pub task_id: usize,
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
            task_id,
            join_waker: Cell::default(),
            payload: RefCell::new(Payload::Pending { fut }),
        }
    }
}

impl<F: Future> RawTask<F> {
    fn insert_waker(&self, cx: &mut Context) {
        self.join_waker.set(Some(cx.waker().clone()));
    }
}

impl<F: Future> Task for RawTask<F>
where
    F::Output: 'static,
{
    /// Aborts a task immediately. Beware not to call this from
    /// the inside a poll function, which might trigger a panic
    /// if a task attempts to abort itself.
    fn abort_in_place(self: Pin<&Self>) {
        let mut payload = self.payload.borrow_mut();
        if let Payload::Pending { .. } = &*payload {
            *payload = Payload::Aborted;
        }
        self.wake_join();
    }
    fn task_id(&self) -> usize {
        self.task_id
    }
    fn poll(self: Pin<&Self>, cx: &mut Context) -> Poll<()> {
        let mut payload = self.payload.borrow_mut();
        if let Payload::Pending { fut } = &mut *payload {
            // SAFETY:
            // we can safely project the pin because the
            // payload future is never moved.
            // Also, safe code can't move the future because
            // RawTask is !Unpin, and its contents are private,
            // so it cannot be moved by safe code.
            let fut = unsafe { Pin::new_unchecked(fut) };

            if let Poll::Ready(output) = fut.poll(cx) {
                *payload = Payload::Ready { output };
                // let's wake the joining task.
                self.wake_join();
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        } else {
            // we return ready so the task can be removed from the queue.
            Poll::Ready(())
        }
    }

    /// This function will schedule the task for cancellation.
    fn abort(self: Pin<&Self>) {
        current_unwrap("abort")
            .executor
            .aborted
            .borrow_mut()
            .push_back(self.task_id);
    }

    fn wake_join(&self) {
        if let Some(waker) = self.join_waker.take() {
            waker.wake();
        }
    }

    /// # Safety
    /// The caller must uphold that the pointer `out: *mut ()` points to a valid
    /// `Poll<F::Output>`, where `F` is the spawned future of the associated task.
    #[track_caller]
    unsafe fn poll_join(self: Pin<&Self>, cx: &mut Context, out: *mut ()) {
        self.insert_waker(cx);
        // we must be careful not to accidentally move the task here.
        let ref mut payload = *self.payload.borrow_mut();

        if !matches!(payload, Payload::Pending { .. }) {
            // we can move anything now that we know the pin ended.
            let payload = replace(payload, Payload::Taken);

            match payload {
                Payload::Ready { output } => {
                    // Safety:
                    // the caller must uphold that the transmuted type is correct.
                    let out: &mut Poll<F::Output> = unsafe {
                        transmute(out)
                        // &mut *(out as *mut std::task::Poll<<F as std::future::Future>::Output>)
                    };
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
}
