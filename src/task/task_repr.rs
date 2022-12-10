use super::raw_task::RawTask;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::hint::unreachable_unchecked;
use std::marker::PhantomPinned;
use std::mem::replace;
use std::panic::resume_unwind;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::thread::panicking;

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
    Panic { error: Box<dyn Any + Send> },
}

impl<F: Future> TaskRepr<F> {
    pub fn new(fut: F) -> Self {
        TaskRepr {
            payload: RefCell::new(Payload::Pending { fut }),
            join_waker: Cell::default(),
            _ph: PhantomPinned,
        }
    }
}
impl<F: Future> Payload<F> {
    pub fn _replace(self: Pin<&mut Self>, payload: Payload<F>) -> Result<Payload<F>, Pin<&mut F>> {
        // Safety: the pending future is never moved
        match unsafe { self.get_unchecked_mut() } {
            Self::Pending { fut } => {
                // Safety: we project the pin
                let fut = unsafe { Pin::new_unchecked(fut) };
                Err(fut)
            }
            other => Ok(replace(other, payload)),
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
    fn poll(self: Pin<&Self>, cx: &mut Context) {
        let mut payload = self.payload.borrow_mut();
        let Payload::Pending { fut } = &mut *payload else { return };
        // Safety: we can safely project the pin because the payload
        // future is never moved.
        let fut = unsafe { Pin::new_unchecked(fut) };

        let Poll::Ready(output) = fut.poll(cx) else { return };
        *payload = Payload::Ready { output };
        // let's wake the joining task.
        self.wake_join_handle();
    }

    fn wake_join_handle(&self) {
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
        let payload = &mut *self
            .payload
            .try_borrow_mut()
            .expect("A task attempted to join iteself. This behavior is not supported.");
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
                Payload::Panic { error } => resume_unwind(error),
                Payload::Aborted => {
                    unreachable!(
                        "attempted to join a task that has been aborted. This is a bug in osiris."
                    )
                }
                // Safety: we already checked for this case
                Payload::Pending { .. } => unsafe { unreachable_unchecked() },
            }
        }
    }

    /// Aborts a task immediately.
    ///
    /// # Panics
    /// If the child task has panicked or the task is already borrowed,
    /// However, this function will avoid to double panic, because it is used
    /// in the drop implementation of `JoinHandle`.
    fn abort(self: Pin<&Self>) {
        let Ok(mut task) = self.payload.try_borrow_mut() else {
            // we don't want to abort the process by
            // double panicking
            if panicking() {
                return;
            }
            unimplemented!("A task attempted to abort itself. This is not supported at the moment, move the JoinHandle to another task or detach it if you don't want it to panic."); 
        };

        if !matches!(&*task, Payload::Panic { .. }) {
            *task = Payload::Aborted;
            self.wake_join_handle();
            return;
        }

        let Payload::Panic{ error } = replace(&mut *task, Payload::Aborted) else {
            // Safety: already checked for the case above
            unsafe { unreachable_unchecked() }
        };
        // we don't want to abort the process by
        // double panicking
        if !panicking() {
            resume_unwind(error);
        }
    }
    fn panic(self: Pin<&Self>, error: Box<dyn Any + Send>) {
        let mut payload = self.payload.borrow_mut();
        *payload = Payload::Panic { error };
    }
}
