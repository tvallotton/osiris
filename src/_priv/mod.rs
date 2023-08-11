//! This is used for internal macros only.
//! Changes to this API are not considered breaking.

pub use join::{Join, JoinWaker};
pub use main::run;

mod join;
mod main;
