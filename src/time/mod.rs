pub use std::time::Duration;
pub use timeout::timeout;

use crate::reactor::op;
pub mod timeout;

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
/// # osiris::block_on(async {
/// use osiris::time::{sleep, Duration};
/// use std::time::Instant;
///
/// let time = Instant::now();
/// let duration = Duration::from_millis(100);
/// sleep(duration).await;
/// assert!(time.elapsed() > duration);
/// # std::io::Result::Ok(()) }).unwrap();
/// ```
///
///
/// # Panics
///
/// This future panics if called outside the context of
/// an osiris runtime.
pub async fn sleep(time: Duration) {
    op::sleep(time).await
}
