//! Asynchronous file and standard stream adaptation.
//!
//! This module contains utility methods and adapter types for input/output to
//! files or standard streams (`Stdin`, `Stdout`, `Stderr`), and
//! filesystem manipulation, for use within (and only within) an Osiris runtime.
//!
//! Unlike nonblocking runtimes, which generally spawn a threadpool to perform
//! blocking file io on, Osiris performs true async file io. This implies that
//! the buffers used by osiris files need to be owned, and cannot work with
//! references.
//!
use std::ffi::CString;
use std::io::Result;
use std::os::unix::prelude::OsStringExt;
use std::path::PathBuf;

pub use dir::{create_dir, remove_dir};
pub use file::{remove_file, File};
pub use metadata::{metadata, symlink_metadata, FileType, Metadata};
pub use open_options::OpenOptions;
pub use read::{read, read_to_string};
pub use symlink::symlink;

mod dir;
mod file;
mod metadata;
mod open_options;
mod read;
mod symlink;

pub(crate) fn cstr(path: impl Into<PathBuf>) -> Result<CString> {
    let path: PathBuf = path.into();
    Ok(CString::new(path.into_os_string().into_vec())?)
}
