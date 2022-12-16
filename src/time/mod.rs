use std::future::{poll_fn, Future};
use std::pin::Pin;
use std::task::{ready, Poll};
pub use std::time::Duration;

/// Waits until `duration` has elapsed. An asynchronous analog to
/// `std::thread::sleep`.
///
/// No work is performed while awaiting on the sleep future to complete.
///
/// # Cancellation
///
/// Canceling a sleep instance is done by dropping the returned future. No additional
/// cleanup work is required.
///
/// # Examples
///
/// Wait 100ms and print "100 ms have elapsed".
///
/// ```
/// use osiris::time::{sleep, Duration};
///
/// #[osiris::main]
/// async fn main() {
///     sleep(Duration::from_millis(100)).await;
///     println!("100 ms have elapsed");
/// }
/// ```
///
///
/// # Panics
///
/// This future panics if called outside the context of
/// an osiris runtime.
pub fn sleep(time: Duration) -> impl Future<Output = ()> + Unpin {
    use crate::shared_driver::submit;
    use io_uring::opcode::Timeout;
    use io_uring::types::Timespec;
    let timespec = Timespec::new()
        .sec(time.as_secs())
        .nsec(time.subsec_nanos());
    let timespec = Box::new(timespec);
    let entry = Timeout::new(&*timespec as *const Timespec)
        .count(u32::MAX)
        .build();
    // Safety: the resource (timespec) was passed to submit
    let mut event = unsafe { submit(entry, timespec) };
    poll_fn(move |cx| {
        ready!(Pin::new(&mut event).poll(cx)).0.unwrap();
        Poll::Ready(())
    })
}
