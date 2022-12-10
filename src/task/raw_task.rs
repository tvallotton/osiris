use std::any::Any;
use std::pin::Pin;
use std::task::Context;

pub(crate) trait RawTask {
    /// this function will wake the join handle that is waiting for the task to
    /// be completed.
    fn wake_join_handle(&self);
    fn abort(self: Pin<&Self>);
    fn poll(self: Pin<&Self>, cx: &mut Context);
    /// This function will check if the task has finished and it will take the value
    /// if 
    ///
    ///
    /// # Safety
    /// The caller must uphold that the pointer `out: *mut ()` points to a valid
    /// memory location of the type `Poll<F::Output>`, where `F` is the spawned
    /// future of the associated task.
    unsafe fn poll_join(self: Pin<&Self>, cx: &mut Context, ptr: *mut ());
    fn panic(self: Pin<&Self>, error: Box<dyn Any + Send>);
}
