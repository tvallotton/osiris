use std::cell::{BorrowMutError, Cell, RefCell};
use std::future::Future;
use std::hint::unreachable_unchecked;
use std::marker::PhantomPinned;
use std::mem::replace;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use super::RawTask;

pub(crate) struct TaskRepr<F: Future> {
    /// Even though strictly speaking cells do not pin project,
    /// we will consider the contents of this cell pinned.
    payload: RefCell<Payload<F>>,
    /// we store here the waker for the JoinHandle.
    pub join_waker: Cell<Option<Waker>>,
    _ph: PhantomPinned,
}

pub(crate) enum Payload<F: Future> {
    Taken,
    Aborted,
    Pending { fut: F },
    Ready { output: F::Output },
}

impl<F: Future> TaskRepr<F> {
    pub fn new(fut: F) -> Self {
        TaskRepr {
            join_waker: Cell::default(),
            payload: RefCell::new(Payload::Pending { fut }),
            _ph: PhantomPinned,
        }
    }
}

impl<F: Future> TaskRepr<F> {
    fn insert_waker(&self, cx: &mut Context) {
        self.join_waker.set(Some(cx.waker().clone()));
    }
}

impl<F: Future> RawTask for TaskRepr<F>
where
    F::Output: 'static,
{
    fn poll(self: Pin<&Self>, cx: &mut Context) -> Poll<()> {
        let mut payload = self.payload.borrow_mut();
        let Payload::Pending { fut } = &mut *payload else {
            // we return ready so the task can be removed from the queue.
            return Poll::Ready(());
        };
        // SAFETY:
        // we can safely project the pin because the
        // payload future is never moved.
        // Also, safe code can't move the future because
        // `TaskRepr` is !Unpin, and its contents are private,
        // so it cannot be moved by safe code.
        let fut = unsafe { Pin::new_unchecked(fut) };

        let Poll::Ready(output) = fut.poll(cx) else {
            return  Poll::Pending;
        };
        *payload = Payload::Ready { output };
        // let's wake the joining task.
        self.wake_join();
        Poll::Ready(())
    }

    fn wake_join(&self) {
        let Some(waker) = self.join_waker.take() else {
            return;
        };
        waker.wake_by_ref();
        self.join_waker.set(Some(waker));
    }

    /// # Safety
    /// The caller must uphold that the pointer `out: *mut ()` points to a valid
    /// memory location of the type `Poll<F::Output>`, where `F` is the spawned
    /// future of the associated task.
    #[track_caller]
    unsafe fn poll_join(self: Pin<&Self>, cx: &mut Context, out: *mut ()) {
        self.insert_waker(cx);
        // we must be careful not to accidentally move the task here.
        let payload = &mut *self.payload.borrow_mut();
        if !matches!(payload, Payload::Pending { .. }) {
            // we can move anything now that we know the pin ended.
            let payload = replace(payload, Payload::Taken);

            match payload {
                Payload::Ready { output } => {
                    let out: *mut Poll<F::Output> = out.cast();
                    // Safety:
                    // the caller must uphold that the transmuted type is correct.
                    unsafe {
                        *out = Poll::Ready(output);
                    }
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

    /// Aborts a task immediately. It may fail if the task
    /// is already borrowed, possibly if it is being polled.
    fn try_abort(self: Pin<&Self>) -> Result<(), BorrowMutError> {
        let mut payload = self.payload.try_borrow_mut()?;
        if let Payload::Pending { .. } = &*payload {
            *payload = Payload::Aborted;
        }
        self.wake_join();
        Ok(())
    }

    fn status(&self) -> &'static str {
        match &*self.payload.borrow() {
            Payload::Aborted => "aborted",
            Payload::Pending { .. } => "pending",
            Payload::Ready { .. } => "ready",
            Payload::Taken => "taken",
        }
    }
}
