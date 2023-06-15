use super::cstr;
use crate::reactor::submit;
use io_uring::opcode::{MkDirAt, UnlinkAt};
use io_uring::types::Fd;
use libc::AT_FDCWD;
use std::io::Result;
use std::path::Path;

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
pub async fn create_dir(path: impl AsRef<Path>) -> Result<()> {
    _create_dir(path.as_ref()).await
}

async fn _create_dir(path: &Path) -> Result<()> {
    let path = cstr(path)?;
    let sqe = MkDirAt::new(Fd(libc::AT_FDCWD), path.as_ptr()).build();
    let (cqe, _) = unsafe { submit(sqe, path).await };
    cqe.map(|_| ())
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
pub async fn remove_dir(path: impl AsRef<Path>) -> Result<()> {
    _remove_dir(path.as_ref()).await
}

async fn _remove_dir(path: &Path) -> Result<()> {
    let path = cstr(path)?;
    let sqe = UnlinkAt::new(Fd(AT_FDCWD), path.as_ptr())
        .flags(libc::AT_REMOVEDIR)
        .build();
    // Safety: the path is protected by submit
    let (cqe, _) = unsafe { submit(sqe, path).await };
    cqe.map(|_| ())
}
