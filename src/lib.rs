#![deny(warnings)]
#![allow(unused_unsafe)]
#![allow(dead_code)]
#![warn(clippy::all)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::cast_possible_truncation)]
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
