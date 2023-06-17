//!
//! Osiris is a completion I/O thread per core runtime intended mainly for developing
//! io-uring applications. At a high level it provides the following components:
//! * A [runtime] for executing asynchronous code, including a task scheduler,
//!    an I/O reactor backed by the operating system's asynchronous API's.
//! * Tools for working with [asynchrnous tasks](task), and [synchronization primitives](sync) for these tasks.
//! * I/O primitives for [networking](net), [file system access](fs), and [timers/timeout operations](time).
//!
//! # Thread per core
//! Osiris follows the thread per core architecture, avoiding thread synchronization whenever possible.
//! This means, that most types are `!Send` and `!Sync`.
//!
//!
//! # Working with tasks
//! In Osiris, tasks can be created using the spawn function, which returns a JoinHandle.
//! The JoinHandle can be used to either join or cancel the task.
//! ## Joining
//! A task can be joined with its parent by awaiting it. The join handle will
//! return whatever object was returned by the child. If the child panics, the
//! panic will be propagated to the parent.
//! ```
//! use osiris::{spawn, time::{sleep, Duration}};
//!
//! #[osiris::main]
//! async fn main() {
//!     let handle  = spawn(async {
//!         sleep(Duration::from_micros(50)).await;
//!         12
//!     });
//!
//!     assert_eq!(12, handle.await);
//! }
//! ```
//! ## Cancellation
//! Osiris follows [structured concurrency](https://en.wikipedia.org/wiki/Structured_concurrency),
//! which discourages orphan tasks. For this reason,  tasks are automatically cancelled when the
//! `JoinHandle` gets dropped. They can also be cancelled explicitly with the [`abort`](task::JoinHandle::abort) method.
//! If the child task panicked, the error will be propagated to the parent during cancellation.
//! Note that the error won't be propagated if the parent is already panicking.
//!```
//! use osiris::{spawn, time::{sleep, Duration}};
//!
//! #[osiris::main]
//! async fn main() {
//!     let handle = spawn(async {
//!         // ...
//!     });
//!
//!     // the task gets cancelled
//!     drop(handle);
//! }
//! ```
//! ## Detached tasks
//! Detached tasks in Osiris can be created using the osiris::detach function.
//! Detached tasks are independent of the parent task and do not require explicit
//! joining or cancellation. They continue to execute independently until completion or termination.
//! ```
//! use osiris::{detach, time::{sleep, Duration}};
//!
//! #[osiris::main]
//! async fn main() {
//!     let handle = detach(async {
//!         // ...
//!     });
//!     
//!     drop(handle);
//!     // task continues execution after being dropped.
//! }
//! ```
#![deny(warnings)]
#![allow(unused_unsafe)]
#![allow(dead_code)]
#![warn(clippy::all)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::ptr_as_ptr)]
#![allow(clippy::len_without_is_empty)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::borrow_as_ptr)]
pub use runtime::block_on;
pub use task::{detach, spawn};
pub mod buf;
#[cfg(target_os = "linux")]
pub mod fs;
#[cfg(target_os = "linux")]
pub mod net;
mod reactor;
pub mod runtime;
pub mod sync;
pub mod task;
#[cfg(target_os = "linux")]
pub mod time;
#[cfg(feature = "macros")]
pub use osiris_macros::{main, test};

fn foo() {
    spawn(async {}).detach();
}
