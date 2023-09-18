use std::any::Any;
use std::mem::replace;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};
use Inner::*;
pub trait Work: Send + Sync {
    fn block(&self);
    fn take(&self, out: &mut dyn Any);
}

pub struct WorkRepr<F, T>(Mutex<Inner<F, T>>);

pub enum Inner<F, T> {
    Queued(F, Waker),
    Running,
    Finished(T),
    Panicked(Box<dyn Any + Send>),
    Taken,
}

impl<F, T> Work for WorkRepr<F, T>
where
    F: FnOnce() -> T + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    fn block(&self) {
        let mut guard = self.0.lock().unwrap();
        guard.block();
    }

    fn take(&self, out: &mut dyn Any) {
        let out: &mut Poll<T> = out.downcast_mut().unwrap();
        let mut guard = self.0.lock().unwrap();

        match replace(&mut *guard, Taken) {
            Panicked(payload) => resume_unwind(payload),
            Taken => unreachable!(),
            value @ (Running | Queued(_, _)) => *guard = value,
            Finished(value) => *out = Poll::Ready(value),
        };
    }
}

impl<F, T> Inner<F, T>
where
    F: FnOnce() -> T,
{
    fn block(&mut self) {
        let Queued(f, waker) = replace(self, Running) else {
            unreachable!("this is ia bug on osiris, we would appreciate if you reported it.")
        };

        waker.wake();

        match catch_unwind(AssertUnwindSafe(f)) {
            Ok(ready) => *self = Finished(ready),
            Err(err) => *self = Panicked(err),
        }
    }
}

pub fn work<F, T>(f: F, waker: Waker) -> Arc<dyn Work + Send + Sync>
where
    F: FnOnce() -> T + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    let inner = Queued(f, waker);
    let work = Mutex::new(inner);
    Arc::new(WorkRepr(work))
}
