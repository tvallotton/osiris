//! Utilities for tracking time.
//!
//! This module provides a couple of async functions for executing scheduling timers:
//!
//! * [`sleep`] is a future that does no work and completes after a minimum duration has elapsed.
//!
//! * [`timeout`](timeout::timeout()): Wraps a future or stream, setting an upper bound to the amount
//!   of time it is allowed to execute. If the future or stream does not
//!   complete in time, then it is canceled and an error is returned.
//!
//! These types are sufficient for handling a large number of scenarios
//! involving time.
//!
//! These types must be used from within the context of the [`Runtime`](crate::runtime::Runtime).
//!
//! # Examples
//!
//! Wait 100ms and print "100 ms have elapsed"
//!
//! ```
//! use std::time::Duration;
//! use osiris::time::sleep;
//!
//! #[osiris::main]
//! async fn main() {
//!     sleep(Duration::from_millis(100)).await;
//!     println!("100 ms have elapsed");
//! }
//! ```
//!
//! Require that an operation takes no more than 1s.
//!
//! ```
//! use osiris::time::{timeout, Duration};
//!
//! async fn long_future() {
//!     // do work here
//! }
//!
//! # async fn dox() {
//! let res = timeout(Duration::from_secs(1), long_future()).await;
//!
//! if res.is_err() {
//!     println!("operation timed out");
//! }
//! # }
//! ```
//!

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
    op::sleep(time).await.unwrap();
}
