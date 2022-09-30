#![warn(clippy::undocumented_unsafe_blocks)]

pub use task::spawn;
#[macro_use]
mod macros;
pub mod runtime;
pub mod task;
