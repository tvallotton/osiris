use crate::runtime::current;
use std::future::Future;
use std::mem::forget;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A function that takes a future and guarantees that the future will
/// be driven to completion before it gets dropped. This is performed by
/// spawning a new task in case the future gets dropped before completion,
/// or by leaking the future, if the task couldn't be spawned.
///
/// # Example
/// ```rust
/// # use std::task::Poll;
/// # use std::future::poll_fn;
/// use osiris::task::complete;
///
/// let will_complete = complete(poll_fn(|cx| {
///     println!("hello world");
///     Poll::Ready(())
/// }));
/// // prints "hello world" if inside a runtime context.
/// // otherwise it forgets the future without running its destructor.
/// drop(will_complete);
/// ```
pub fn complete<F: Future + 'static + Unpin>(f: F) -> impl Future<Output = F::Output> {
    Completion { future: Some(f) }
}

/// A future that is guaranteed to be driven to completion
/// before it gets dropped.
struct Completion<F: Future + 'static + Unpin> {
    future: Option<F>,
}

impl<F: Future + 'static + Unpin> Future for Completion<F> {
    type Output = F::Output;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let future = self.future.as_mut().unwrap();
        Pin::new(future).poll(cx)
    }
}

impl<F: Future + 'static + Unpin> Drop for Completion<F> {
    fn drop(&mut self) {
        if let Some(future) = self.future.take() {
            let future = future;
            let future = complete(future);
            if let Some(rt) = current() {
                rt.spawn(future);
            } else {
                forget(future);
            }
        }
    }
}
