//!
//! Osiris is a completion I/O thread per core runtime intended mainly for developing
//! io-uring applications. At a high level it provides the following components:
//! * A [runtime] for executing asynchronous code, including a task scheduler,
//!    an I/O reactor backed by the operating system's asynchronous API's.
//! * Tools for working with [asynchrnous tasks](task), and [synchronization primitives](sync) for these tasks.
//! * I/O primitives for [networking](net), [file system access](fs), and [timers/timeout operations](time).
//!
//! # Share-nothing
//! Osiris follows a share-nothing architecture, avoiding thread synchronization whenever possible.
//! This means, that most types are `!Send` and `!Sync`. By default, when using the [`main`] macro the
//! application is single threaded. It can be made multithreaded by settning the `scale` parameter:
//! ```no_run
//! // this will spawn one thread per core and set
//! // the affinity for each thread to a separate CPU.
//! #[osiris::main(scale = true)]
//! async fn main() {
//!     // ...
//! }
//! ```
//! To restart threads if they panic or in [`Err`] the `restart` configuration can be set.
//! It won't restart if the thread exists normally with a successful [exit code](std::process::ExitCode).
//! ```
//! #[osiris::main(scale = true, restart = true)]
//! async fn main() {
//!     // ...
//! }
//! ```
//! If more control is needed over the number of threads, it can be specified explicitly
//! ```no_run
//! // this will spawn 4 threads and set the affinity of each
//! // thread to different core if possible
//! #[osiris::main(scale = 4)]
//! async fn main() {
//!     // ...
//! }
//! ```
//! Note that scaling the application will create identical parallel replicas of the main task, which is useful for a
//! concurrent server, but not as much for clients. This shouldn't be confused with how  work stealing runtimes work
//! (e.g. [`tokio`](https://docs.rs/tokio/latest/tokio/)), that will spawn a pool of worker threads, but the main task will remain unique.
//!
//! # Working with tasks
//! In Osiris, tasks can be created using the [`spawn`] function, which returns a [`JoinHandle`](task::JoinHandle).
//! The JoinHandle can be used to either join or cancel the task.
//! ## Joining
//! A task can be joined with its parent by awaiting it. The join handle will
//! return whatever object was returned by the child. If the child panics, the
//! panic will be propagated to the parent.
//! ```no_run
//! use osiris::{spawn, time::{sleep, Duration}};
//!
//! #[osiris::main]
//! async fn main() {
//!     let handle = spawn(async {
//!         sleep(Duration::from_millis(1500)).await;
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
//! Detached tasks in Osiris can be created using the [`osiris::detach`] function.
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
//! # Async I/O
//! Osiris is a completion based async runtime, which means it has stricter requirements
//! for what kinds of buffers can be used for I/O. Specifically, it cannot work with non-'static
//! references, only owned buffers or 'static references.
//!
//! Generally, all I/O APIs will return back the buffer that was fed as an input.
//! In order to work with slices the [`slice`](buf::IoBuf::slice) method can be used.
//!
//! ## File system
//! Unlike nonblocking based runtimes, Osiris offers true asynchronous file I/O
//! ```
//! use osiris::fs::read_to_string;
//!
//! #[osiris::main]
//! async fn main() -> std::io::Result<()> {
//!     let data = read_to_string("./Cargo.toml").await?;
//!     assert!(data.contains("osiris"));
//!     Ok(())
//! }
//! ```
//! ## Networking
//! Osiris offers networking types analogous to the ones found in [`std::net`].
//! ```no_run
//! use osiris::net::TcpStream;
//!
//! #[osiris::main]
//! async fn main() -> std::io::Result<()> {
//!     let mut stream = TcpStream::connect("www.example.com:80").await?;
//!     stream
//!         .write_all(b"GET / HTTP/1.1\r\nHost: www.example.com\r\n\r\n")
//!         .await.0?;
//!     let buf = vec![0; 256];
//!     let (n, buf) = stream.read(buf).await;
//!     let response = &buf[..n?];
//!     assert!(response .starts_with(b"HTTP/1.1 200 OK"));
//!     stream.close().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Timers
//! Any future can be cancelled after a timeout with the [`timeout`](`time::timeout::timeout`) function.
//! ```
//! use osiris::time::{timeout, sleep, Duration};
//!
//! #[osiris::main]
//! async fn main() {
//!     let future = async {
//!         // this is going to take too long
//!         sleep(Duration::from_secs(1000)).await
//!     };
//!     let res = timeout(Duration::from_micros(100), future).await;
//!     assert!(res.is_err(), "{res:?}");
//! }
//! ```
//!
//! ## Synchronization
//! Osiris offers atomic free synchronization primitives. These primitives are designed to synchronize tasks
//! instead of threads. This means that they are cheaper but they do not implement Send or Sync.
//! ```
//! use osiris::sync::mpmc::channel;
//! use osiris::join;
//!
//! #[osiris::main]
//! async fn main() {
//!     let (tx, rx) = channel(1);
//!     let (_, r) = join!(tx.send(42), rx.recv());
//!     assert_eq!(42, r.unwrap())
//! }
//! ```
//!
//! # Tokio compatibility
//! Osiris offers the compile time feature `tokio_compat` to enable support for Tokio.
//! ### Examples
//! Using Tokio futures from an Osiris executor
//! ```no_run
//! # #![cfg(features = "tokio_compat")]
//! use tokio::time::{sleep, Duration};
//!
//! #[osiris::main]
//! async fn main() {
//!     sleep(Duration::from_secs(1)).await;
//! }
//! ```
//!
//! Using Osiris futures from a Tokio executor. Note that the Tokio executor requires a local set to be able to run
//! Osiris futures. This is because Osiris needs to be able to spawn a task to run the future on Tokio, but
//! it needs to spawn a `!Send` task, which can only be done with `LocalSet`.
//! ```no_run
//! # #![cfg(features = "tokio_compat")]
//! use osiris::time::{sleep, Duration};
//!
//! #[tokio::main]
//! async fn main() {
//!     let local = tokio::task::LocalSet::new();
//!
//!     local.run_until(async {
//!         sleep(Duration::from_secs(1)).await;
//!     });
//! }
//! ```
//!

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

extern crate self as osiris;

#[cfg(feature = "macros")]
pub use osiris_macros::{main, test};
pub use runtime::block_on;
pub use task::{detach, spawn};

mod utils;

pub mod _priv;
pub mod buf;

pub mod fs;
pub mod net;
mod reactor;
pub mod runtime;
pub mod sync;
pub mod task;
pub mod time;
#[cfg(test)]
mod type_assertions;

// delete this test to get rid of the stack overflow
#[cfg(test)]
#[test]

fn stack_overflow() {}
