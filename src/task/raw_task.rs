use std::any::Any;
use std::pin::Pin;
use std::task::{Context, Poll};

pub(crate) trait RawTask {
    fn wake_join(&self);
    fn abort(self: Pin<&Self>);
    fn poll(self: Pin<&Self>, cx: &mut Context) -> Poll<()>;
    unsafe fn poll_join(self: Pin<&Self>, cx: &mut Context, ptr: *mut ());
    fn panic(self: Pin<&Self>, error: Box<dyn Any + Send>);
}
