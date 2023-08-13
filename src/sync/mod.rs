//! Synchronization primitives for tasks.
//!
//! Note that unlike std's, or tokio's synchronization primitives, osiris's
//! are designed to be used across tasks, not across threads, so they do not implement
//! the `Send` and `Sync` traits. Synchronizing tasks is cheaper than
//! synchronizing threads, so when working with osiris tasks, these implementations are a
//! good choice.
//!

pub use mutex::{Error as MutexError, Guard as MutexGuard, Mutex};

pub mod mpmc;
pub mod mutex;
