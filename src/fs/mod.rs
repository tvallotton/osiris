#[cfg(feature = "unstable")]
pub use dir::create_dir;
pub use file::File;
pub use metadata::{metadata, Metadata};
pub use open_options::OpenOptions;

#[cfg(feature = "unstable")]
mod dir;
mod file;
mod metadata;
mod open_options;
