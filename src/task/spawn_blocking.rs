use crate::runtime::{current_unwrap, ThreadPool, THREAD_POOL};

use super::JoinHandle;

/// Runs the provided closure on a thread where blocking is acceptable.
///
/// In general, issuing a blocking call or performing a lot of compute in a
/// future without yielding is problematic, as it may prevent the executor from
/// driving other futures forward. This function runs the provided closure on a
/// thread dedicated to blocking operations. See the [CPU-bound tasks and
/// blocking code][blocking] section for more information.
///
/// Osiris will spawn more blocking threads when they are requested through this
/// function until the upper limit configured on [`Config`](crate::runtime::Config) is reached.
/// After reaching the upper limit, the tasks are put in a queue.
/// The thread limit is very large by default, because `spawn_blocking` is often
/// used for various kinds of IO operations that cannot be performed
/// asynchronously.  When you run CPU-bound code using `spawn_blocking`, you
/// should keep this large upper limit in mind. When running many CPU-bound
/// computations, a semaphore or some other synchronization primitive should be
/// used to limit the number of computation executed in parallel. Specialized
/// CPU-bound executors, such as [rayon], may also be a good fit.
///
/// This function is intended for non-async operations that eventually finish on
/// their own. If you want to spawn an ordinary thread, you should use
/// [`thread::spawn`] instead.
///
/// Closures spawned using `spawn_blocking` cannot be cancelled abruptly; there
/// is no standard low level API to cause a thread to stop running.  However,
/// a useful pattern is to pass some form of "cancellation token" into
/// the thread.  This could be an [`AtomicBool`] that the task checks periodically.
/// Another approach is to have the thread primarily read or write from a channel,
/// and to exit when the channel closes; assuming the other side of the channel is dropped
/// when cancellation occurs, this will cause the blocking task thread to exit
/// soon after as well.
//
/// # Related APIs and patterns for bridging asynchronous and blocking code
///
/// In simple cases, it is sufficient to have the closure accept input
/// parameters at creation time and return a single value (or struct/tuple, etc.).
///
/// Another option is [`SyncIoBridge`] for cases where the synchronous context
/// is operating on byte streams.  For example, you might use an asynchronous
/// HTTP client such as [hyper] to fetch data, but perform complex parsing
/// of the payload body using a library written for synchronous I/O.
///
///
/// [blocking]: ../index.html#cpu-bound-tasks-and-blocking-code
/// [rayon]: https://docs.rs/rayon
/// [`mpsc channel`]: crate::sync::mpsc
/// [`SyncIoBridge`]: https://docs.rs/tokio-util/latest/tokio_util/io/struct.SyncIoBridge.html
/// [hyper]: https://docs.rs/hyper
/// [`thread::spawn`]: fn@std::thread::spawn
/// [`shutdown_timeout`]: fn@crate::runtime::Runtime::shutdown_timeout

/// [`AtomicBool`]: struct@std::sync::atomic::AtomicBool
///
/// # Examples
///
/// Pass an input value and receive result of computation:
///
/// ```
/// use osiris::task;
///
/// # async fn docs() -> Result<(), Box<dyn std::error::Error>>{
/// // Initial input
/// let mut v = "Hello, ".to_string();
/// let res = task::spawn_blocking(move || {
///     // Stand-in for compute-heavy work or using synchronous APIs
///     v.push_str("world");
///     // Pass ownership of the value back to the asynchronous context
///     v
/// }).await;
///
/// // `res` is the value returned from the thread
/// assert_eq!(res.as_str(), "Hello, world");
/// # Ok(())
/// # }
/// ```
///
pub fn spawn_blocking<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    let rt = current_unwrap("spawn_blocking");
    THREAD_POOL
        .get_or_init(|| ThreadPool::new(rt.config))
        .spawn_blocking(f)
}
