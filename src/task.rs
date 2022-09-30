use crate::runtime::current_unwrap;
pub use join_handle::JoinHandle;
use std::{
    any::Any,
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};
pub use yield_now::yield_now;

use self::raw_task::RawTask;
mod join_handle;
mod raw_task;
mod yield_now;

pub fn spawn<F>(future: F) -> JoinHandle<<F as Future>::Output>
where
    F: Future + 'static,
{
    current_unwrap("spawn").spawn(future)
}

pub(crate) trait Task {
    fn abort(self: Pin<&Self>);
    fn poll(self: Pin<&Self>, cx: &mut Context) -> Poll<()>;
    fn poll_join(self: Pin<&Self>, cx: &mut Context, output: &mut dyn Any);
}

impl dyn Task {
    pub(crate) fn new<F: Future + 'static>(task_id: usize, fut: F) -> Pin<Rc<dyn Task>> {
        Rc::pin(RawTask::new(task_id, fut))
    }
}
