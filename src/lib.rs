#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]
// #![deny(warnings)]
#![allow(unused_unsafe)]
#![allow(dead_code)]
#![warn(clippy::pedantic)]
#![warn(clippy::undocumented_unsafe_blocks)]
#![warn(unsafe_op_in_unsafe_fn)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::ptr_as_ptr)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::borrow_as_ptr)]
pub use runtime::block_on;
pub use task::{detach, spawn};
pub mod buf;
#[cfg(target_os = "linux")]
pub mod fs;
pub mod runtime;
pub mod shared_driver;
pub mod sync;
pub mod task;
#[cfg(target_os = "linux")]
pub mod time;
