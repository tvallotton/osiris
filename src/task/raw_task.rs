use std::any::Any;
use std::pin::Pin;
use std::task::Context;

pub(crate) trait RawTask {
    /// this function will wake the join handle that is waiting for the task to
    /// be completed.
    fn wake_join_handle(&self);
    /// This function is used to abort a task in place. Currently self aborting tasks
    /// are not supported.
    fn abort(self: Pin<&Self>);
    /// This function is used to poll the future and drive it to completion. This method
    /// is called by the executor.
    fn poll(self: Pin<&Self>, cx: &mut Context);
    /// This function will check if the task has finished and it will take the value
    /// in that case. This method is called by the join handle when it's polled.
    ///
    /// # Safety
    /// The caller must uphold that the pointer `out: *mut ()` points to a valid
    /// memory location of the type `Poll<F::Output>`, where `F` is the spawned
    /// future of the associated task.
    unsafe fn poll_join(self: Pin<&Self>, cx: &mut Context, ptr: *mut ());

    /// This function is used to register that the task has panicked so it can
    /// be propagated to the join handle. This function is called by the executor
    /// if the task panics.
    fn panic(self: Pin<&Self>, error: Box<dyn Any + Send>);
}
