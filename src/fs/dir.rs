use super::cstr;
use crate::reactor::op::{self, unlink_at};
use std::io::Result;
use std::path::PathBuf;

/// Creates a new, empty directory at the provided path
///
/// # Errors
///
/// This function will return an error in the following situations, but is not
/// limited to just these cases:
///
/// * User lacks permissions to create directory at `path`.
/// * A parent of the given path doesn't exist.
/// * `path` already exists.
///
/// # Examples
///
/// ```no_run
/// # osiris::block_on(async {
/// use osiris::fs;
///
/// fs::create_dir("/some/dir").await?;
/// # std::io::Result::Ok(()) }).unwrap();
/// ```
pub async fn create_dir(path: impl Into<PathBuf>) -> Result<()> {
    let path = cstr(path)?;
    op::mkdir_at(path).await
}

/// Removes an empty directory.
///
/// # Platform-specific behavior
///
/// This function currently corresponds to the `unlinkat` function on linux
/// and the `RemoveDirectory` function on Windows.
/// Note that, this may change in the future.
///
/// # Errors
///
/// This function will return an error in the following situations, but is not
/// limited to just these cases:
///
/// * `path` doesn't exist.
/// * `path` isn't a directory.
/// * The user lacks permissions to remove the directory at the provided `path`.
/// * The directory isn't empty.
///
/// # Examples
///
/// ```no_run
/// # osiris::block_on(async {
/// use osiris::fs;
///
/// fs::remove_dir("/some/dir").await?;
/// # std::io::Result::Ok(()) }).unwrap();
/// ```
pub async fn remove_dir(path: impl Into<PathBuf>) -> Result<()> {
    let path = cstr(path)?;
    unlink_at(path, libc::AT_REMOVEDIR).await
}
