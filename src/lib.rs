// #![deny(warnings)]
#![allow(unused_unsafe)]
#![allow(dead_code)]
// #![warn(clippy::undocumented_unsafe_blocks)]
// #![warn(unsafe_op_in_unsafe_fn)]

pub use runtime::block_on;
pub use task::spawn;

#[macro_use]
mod macros;

pub mod fs;
mod hasher;
pub mod io;
pub mod runtime;
pub mod shared_driver;
pub mod task;
