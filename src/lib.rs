#![forbid(clippy::undocumented_unsafe_blocks)]
#![forbid(unsafe_op_in_unsafe_fn)]

use std::task::Context;

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
