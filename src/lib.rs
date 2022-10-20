#![warn(clippy::undocumented_unsafe_blocks)]
#![warn(unsafe_op_in_unsafe_fn)]

pub use runtime::block_on;
pub use task::spawn;

#[macro_use]
mod macros;

pub mod driver;
pub mod fs;
mod hasher;
pub mod io;
pub mod runtime;
pub mod task;
