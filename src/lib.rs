#![allow(incomplete_features)]
#![deny(warnings)]
#![allow(unused_unsafe)]
#![allow(dead_code)]
#![warn(clippy::pedantic)]
#![warn(clippy::undocumented_unsafe_blocks)]
#![warn(unsafe_op_in_unsafe_fn)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::ptr_as_ptr)]
#![allow(clippy::borrow_as_ptr)]

mod hasher;

pub mod runtime;
pub mod shared_driver;
pub mod task;
