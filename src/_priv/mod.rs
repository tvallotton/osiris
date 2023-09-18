#![doc(hidden)]
//! This is used for internal macros only.
//! Changes to this API are not considered breaking.

pub use join::Join;
pub(crate) use join_waker::cast;
pub use join_waker::JoinWaker;
pub use main::run;
pub use try_join::TryJoin;

mod join;
mod join_waker;
mod main;
mod try_join;
