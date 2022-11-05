#[cfg(target_os = "linux")]
use io_uring::IoUring;

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Config {
    /// Sets the number of scheduler ticks after which the scheduler will poll for
    /// external events (timers, I/O, and so on).
    ///
    /// A scheduler "tick" corresponds to one `poll` invocation on a task. By default,
    /// the event interval is `61`. Which means that at most `61` futures will be polled
    /// before polling for events.
    ///
    /// Setting the event interval determines the effective "priority" of delivering
    /// these external events (which may wake up additional tasks, or cancel aborted tasks),
    /// compared to executing tasks that are currently ready to run. A smaller value is useful
    /// when tasks frequently spend a long time in polling,  which can result in overly
    /// long delays picking up I/O events. Conversely, picking up new events requires extra
    /// synchronization and syscall overhead, so if tasks generally complete their polling
    /// quickly, a higher event interval will minimize that overhead while still keeping the
    /// scheduler responsive to events.
    ///
    /// This number is intentionally set to a prime number close to a power of 2 so to avoid
    /// unintentional synchronizations with events that may occur at a predictable frequency.
    ///
    pub event_interval: u32,
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    /// The number of entries used in the io-uring/io-ring ringbuffer.
    /// This field determines the maximum number of concurrent io operations
    /// that can be submitted to the kernel at a time. It defaults to 2048.
    /// This value cannot be greater than 4096.
    pub ring_entries: u32,
    /// Determines whether the kernel will be notified for events, or whether it will be continuously
    /// polling for events. By default this value is set to `Notify`
    #[cfg(target_os = "linux")]
    pub mode: Mode,
}
/// Determines whether the kernel will be notified for events, or whether it will be continuously
/// polling for events.
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

impl Default for Config {
    fn default() -> Self {
        Config {
            event_interval: 61,
            #[cfg(target_os = "linux")]
            ring_entries: 2048,
            #[cfg(target_os = "linux")]
            mode: Mode::default(),
        }
    }
}

impl Config {
    pub const DEFAULT_WAKERS: usize = 2048;

    #[cfg(target_os = "linux")]
    pub fn io_uring(self) -> std::io::Result<IoUring> {
        let mut builder = IoUring::builder();
        if let Mode::Polling { idle_timeout } = self.mode {
            builder.setup_sqpoll(idle_timeout);
        }
        builder.build(self.ring_entries)
    }
}
