use super::executor::Executor;
use super::Runtime;
use crate::reactor::Reactor;
use std::rc::Rc;
use std::time::Duration;

#[cfg(target_os = "linux")]
use io_uring::IoUring;

/// Configuration struct for an osiris runtime.
/// The default values for configuration options should not be considered stable.
/// # Example
/// ```rust
/// # use osiris::runtime::{Config, Runtime, Mode};
/// # fn __() -> Result<(), std::io::Error> {
/// // we override the values we want to change
/// let config = Config {
///     #[cfg(target_os = "linux")]
///     mode: Mode::Polling { idle_timeout: 100 },
///     .. Config::default()
/// };
///
/// let runtime = config.build()?;
/// runtime.block_on(async {
///     /* ... */
/// });
/// # Ok(())}
/// ```
#[derive(Clone, Debug)]
pub struct Config {
    /// Sets the number of entries for the submission queue. The completion queue will be set
    /// to be at least double the size of the submission queue. It determines the maximum
    /// number of events that can be submitted to the kernel at once. It defaults to 128.
    ///
    /// On the one hand, a big value will minimize the number of expensive system calls
    /// required to perform the events, thus, maximizing throughput. On the other hand,
    /// a small value will minimize the amount of time the runtime takes to submit the
    /// events to the kernel, minimizing latency.
    ///
    /// Overall, unless the application is dealing with a very heavy load of I/O events,
    /// a smaller value will likely be fine.
    ///
    /// This value is silently capped to 4096.
    pub queue_entries: u32,
    #[cfg(target_os = "linux")]
    /// Determines whether the kernel will be notified for events, or whether it will be continuously
    /// polling for them. By default this value is set to `Notify`.
    pub mode: Mode,
    /// Determines the initial allocation size. When the runtime is expected to run for a
    /// long period of time, or it is expected to manage millions of tasks then a bigger value
    /// is better. When the runtime is going to be used for a single io-event then a smaller value
    /// is best. It defaults to 1024.
    ///
    /// Specifically, this determines the initial allocation for the executor queue.
    pub init_capacity: usize,

    /// Configuration for the shared thread pool. Note that the threadpool
    pub thread_pool: ThreadPoolConfig,

    // Do not use this field. Changes related to this field are considered breaking changes.
    // To construct a value of this type use `Config::default()`. Additional fields may be added
    // any time
    #[doc(hidden)]
    pub do_not_use_this_field: (),
}

/// Determines whether the kernel will be notified for events, or whether it will be continuously
/// polling for them.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub enum Mode {
    /// The kernel will be notified of submissions with a context switch.
    /// This configuration is best when a moderate amount of IO is expected.
    #[default]
    Notify,
    /// The kernel will poll the io-uring submission queue, which skips
    /// a system call notification. This configuration should only be used
    /// if a really big amount of IO is expected.
    Polling {
        /// The maximum amount of time the OS thread will poll before
        /// sleeping. It is messured in milliseconds. It is recommended
        /// to have this be a low value to minimize CPU consumption.
        idle_timeout: u32,
    },
}

#[derive(Clone, Debug)]
pub struct ThreadPoolConfig {
    /// Max amount of time a worker may be idle before it exits.
    /// It defaults to 10s.
    pub idle_timeout: Duration,
    /// Max amount of time an element can remain in the queue before
    /// a new worker is spawned. This timeout will be ignored if the
    /// maximum number of workers is reached. It defaults to 250ms.
    pub wait_timeout: Duration,
    /// Max number of workers that can be spawned by the threadpool.
    /// It defaults to 256.
    pub max_workers: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            queue_entries: 128,
            #[cfg(target_os = "linux")]
            mode: Mode::default(),
            init_capacity: 1024,
            thread_pool: ThreadPoolConfig::default(),
            do_not_use_this_field: (),
        }
    }
}

impl Default for ThreadPoolConfig {
    fn default() -> Self {
        ThreadPoolConfig {
            idle_timeout: Duration::from_secs(2),
            wait_timeout: Duration::from_millis(250),
            max_workers: 256,
        }
    }
}

impl Config {
    /// Creates the configured Runtime.
    /// The returned Runtime instance is ready to spawn tasks.
    ///
    /// # Errors
    /// If the async primitives could not be instantiated.
    pub fn build(self) -> std::io::Result<Runtime> {
        let executor = Rc::new(Executor::new(self.clone())?);
        let reactor = Reactor::new(self.clone())?;
        let rt = Runtime {
            config: self,
            executor,
            reactor,
        };
        Ok(rt)
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn io_uring(self) -> std::io::Result<IoUring> {
        let mut builder = IoUring::builder();
        if let Mode::Polling { idle_timeout } = self.mode {
            builder.setup_sqpoll(idle_timeout);
        }
        builder.build(self.queue_entries.min(4096))
    }
}
