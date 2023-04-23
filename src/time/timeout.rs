use std::future::{poll_fn, Future};
use std::pin::Pin;
use std::task::Poll::*;
use std::time::Duration;

use super::sleep;
pub struct Error(());

/// Requires a `Future` to complete before the specified duration has elapsed.
///
/// If the future completes before the duration has elapsed, then the completed
/// value is returned. Otherwise, an error is returned and the future is
/// canceled.
///
/// Note that the timeout is checked before polling the future, so if the future
/// does not yield during execution then it is possible for the future to complete
/// and exceed the timeout _without_ returning an error.
///
/// This function returns a future whose return type is [`Result`]`<T,`[`Error`]`>`, where `T` is the
/// return type of the provided future.
///
/// [`Result`]: std::result::Result
/// [`Error`]: crate::time::timeout::Error
///
/// # Cancellation
///
/// Cancelling a timeout is done by dropping the future. No additional cleanup
/// or other work is required.
///
/// # Panics
/// This function panics if polled outside a runtime context.
///
pub async fn timeout<F: Future>(mut f: F, dur: Duration) -> Result<F::Output, Error> {
    let mut sleep = sleep(dur);
    poll_fn(move |cx| {
        // Safety: we project the Pin
        let f = unsafe { Pin::new_unchecked(&mut f) };
        let sleep = unsafe { Pin::new(&mut sleep) };

        if sleep.poll(cx).is_ready() {
            return Ready(Err(Error(())));
        }

        if let Ready(val) = f.poll(cx) {
            return Ready(Ok(val));
        }
        Pending
    })
    .await
}

#[test]
fn timeout_() {
    crate::block_on(async {
        let future = sleep(Duration::from_millis(50));

        let out = timeout(future, Duration::from_millis(100)).await;
        assert!(out.is_ok());
        let future = sleep(Duration::from_millis(50));

        let out = timeout(future, Duration::from_millis(10)).await;
        assert!(out.is_err());
    })
    .unwrap();
}
