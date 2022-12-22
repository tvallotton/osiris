use crate::shared_driver::submit;
use io_uring::opcode::MkDirAt;
use io_uring::types::Fd;
use libc::AT_FDCWD;
use std::ffi::CString;
use std::io::{Error, Result};
use std::os::unix::prelude::OsStrExt;
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
/// use osiris::fs;
///
/// #[osiris::main]
/// async fn main() -> std::io::Result<()> {
///     fs::create_dir("/some/dir").await?;
///     Ok(())
/// }
/// ```
#[cfg(feature = "unstable")]
pub async fn create_dir(path: impl AsRef<Path>) -> Result<()> {
    _create_dir(path.as_ref()).await
}

#[cfg(feature = "unstable")]
async fn _create_dir(path: &Path) -> Result<()> {
    let path = CString::new(path.as_os_str().as_bytes()).unwrap();
    let sqe = MkDirAt::new(Fd(libc::AT_FDCWD), path.as_ptr()).build();
    let (cqe, _) = unsafe { submit(sqe, path).await };
    if cqe?.result() < 0 {
        Err(Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Removes an empty directory.
///
/// # Platform-specific behavior
///
/// This function currently corresponds to the `unlinkat` function on linux
/// and the `RemoveDirectory` function on Windows.
/// Note that, this [may change in the future][changes].
///
/// [changes]: io#platform-specific-behavior
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
/// use osiris::fs;
///
/// #[osiris::main]
/// async fn main() -> std::io::Result<()> {
///     fs::remove_dir("/some/dir").await?;
///     Ok(())
/// }
/// ```
#[cfg(feature = "unstable")]
pub async fn remove_dir(path: impl AsRef<Path>) -> Result<()> {
    _remove_dir(path.as_ref()).await
}

async fn _remove_dir(path: &Path) -> Result<()> {
    let path = CString::new(path.as_os_str().as_bytes()).unwrap();
    let sqe = io_uring::opcode::UnlinkAt::new(Fd(AT_FDCWD), path.as_ptr())
        .flags(libc::AT_REMOVEDIR)
        .build();
    // Safety: the path is protected by submit
    let (res, _) = unsafe { submit(sqe, path).await };
    let cqe = res?;
    if cqe.result() < 0 {
        return Err(Error::from_raw_os_error(-cqe.result()));
    }
    Ok(())
}

// pub async fn remove_dir_all(path: impl AsRef<Path>) -> Result<()> {
//     _remove_dir_all(path.as_ref()).await
// }

// // pub async fn _remove_dir_all(path: &Path) -> Result<()> {
// //     let path = CString::new(path.as_os_str().as_bytes()).unwrap();

// // }
