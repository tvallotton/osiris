use std::mem::transmute;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Wake, Waker};

pub struct JoinWaker<const I: u8>(pub Waker, pub AtomicU64);

impl<const I: u8> JoinWaker<I> {
    pub fn new(waker: Waker) -> Self {
        JoinWaker(waker, AtomicU64::new(!0))
    }
}

pub fn cast<const I: u8, const J: u8>(arc: Arc<JoinWaker<I>>) -> Arc<JoinWaker<J>> {
    unsafe { transmute(arc) }
}

impl<const I: u8> Wake for JoinWaker<I> {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.wake_by_ref();
        self.1.fetch_or(1 << I, Ordering::Release);
    }
}
