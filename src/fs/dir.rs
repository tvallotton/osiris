use crate::shared_driver::submit;
use io_uring::opcode::MkDirAt;
use io_uring::types::Fd;
use std::ffi::CString;
use std::io::Error;
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
/// * A parent of the given path doesn't exist. (To create a directory and all
///   its missing parents at the same time, use the [`create_dir_all`]
///   function.)
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
pub async fn create_dir(path: impl AsRef<Path>) -> std::io::Result<()> {
    _create_dir(path.as_ref()).await
}

#[cfg(feature = "unstable")]
async fn _create_dir(path: &Path) -> std::io::Result<()> {
    let path = CString::new(path.as_os_str().as_bytes()).unwrap();
    let sqe = MkDirAt::new(Fd(libc::AT_FDCWD), path.as_ptr()).build();
    let (cqe, _) = unsafe { submit(sqe, path).await };
    if cqe?.result() < 0 {
        Err(Error::last_os_error())
    } else {
        Ok(())
    }
}
