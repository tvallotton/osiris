pub use join_handle::JoinHandle;
use std::{
    any::Any,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};
pub use yield_now::yield_now; 
use crate::runtime::waker::waker;

use self::raw_task::{Payload, RawTask};
mod join_handle;
mod yield_now;
mod raw_task;

pub(crate) trait Task {
    fn wake(self: Pin<&mut Self>);
    fn abort(self: Pin<&mut Self>);
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()>;
    fn poll_join(self: Pin<&mut Self>, cx: &mut Context, output: &mut dyn Any);
}

impl dyn Task {
    pub(crate) fn new<F: Future + 'static>(task_id: usize, fut: F) -> Pin<Rc<dyn Task>> {
        Rc::pin(RawTask {
            task_id,
            join_waker: None,
            payload: Payload::Pending { fut },
        })
    }
}
