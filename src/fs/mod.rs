use std::ffi::CString;
use std::io::Result;
use std::os::unix::prelude::OsStrExt;
use std::path::Path;

#[cfg(feature = "unstable")]
pub use dir::{create_dir, remove_dir};
pub use file::File;
pub use metadata::{metadata, Metadata};
pub use open_options::OpenOptions;

#[cfg(feature = "unstable")]
mod dir;
mod file;
mod metadata;
mod open_options;
mod read;

fn cstr(path: &Path) -> Result<CString> {
    Ok(CString::new(path.as_os_str().as_bytes())?)
}
