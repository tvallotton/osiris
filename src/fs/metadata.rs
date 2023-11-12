#![allow(unreachable_code)]
use super::cstr;
use crate::reactor::op;
#[cfg(target_os = "linux")]
use crate::utils::{statx, statx_timestamp};
use libc::{mode_t, AT_SYMLINK_NOFOLLOW, S_IFDIR, S_IFLNK, S_IFMT, S_IFREG};
use std::io::{self, Error, Result};
use std::path::Path;
use std::time::{Duration, SystemTime};

/// Given a path, query the file system to get information about a file,
/// directory, etc.
///
/// This function will traverse symbolic links to query information about the
/// destination file.
///
/// # Platform-specific behavior
///
/// This function currently corresponds to the `statx` function on Unix
/// and the `GetFileInformationByHandle` function on Windows.
/// Note that, this may change in the future.
///
/// # Errors
///
/// This function will return an error in the following situations, but is not
/// limited to just these cases:
///
/// * The user lacks permissions to perform `metadata` call on `path`.
/// * `path` does not exist.
///
/// # Examples
///
/// ```rust,no_run
/// use osiris::fs;
///
/// #[osiris::main]
/// async fn main() -> std::io::Result<()> {
///     let attr = fs::metadata("/some/file/path.txt").await?;
///     // inspect attr ...
///     Ok(())
/// }
/// ```
pub async fn metadata(path: impl AsRef<Path>) -> Result<Metadata> {
    _metadata(path.as_ref(), 0).await
}

/// Query the metadata about a file without following symlinks.
///
/// # Platform-specific behavior
///
/// This function currently corresponds to the `lstat` function on Unix
/// and the `GetFileInformationByHandle` function on Windows.
/// Note that, this may change in the future.
///
/// # Errors
///
/// This function will return an error in the following situations, but is not
/// limited to just these cases:
///
/// * The user lacks permissions to perform `metadata` call on `path`.
/// * `path` does not exist.
///
/// # Examples
///
/// ```rust,no_run
/// use osiris::fs;
///
/// #[osiris::main]
/// async fn main() -> std::io::Result<()> {
///     let attr = fs::symlink_metadata("/some/file/path.txt").await?;
///     // inspect attr ...
///     Ok(())
/// }
/// ```
pub async fn symlink_metadata(path: impl AsRef<Path>) -> Result<Metadata> {
    _metadata(path.as_ref(), AT_SYMLINK_NOFOLLOW).await
}

async fn _metadata(path: &Path, flags: i32) -> std::io::Result<Metadata> {
    let path = cstr(path)?;
    let statx = op::statx(libc::AT_FDCWD, Some(path), flags).await?;
    Ok(Metadata { statx })
}

/// Metadata information about a file.
///
/// This structure is returned from the [`metadata`] function
/// or method and represents known metadata about a file such
/// as its permissions, size, modification
/// times, etc.
pub struct Metadata {
    pub(crate) statx: statx,
}

/// A structure representing a type of file with accessors for each file type.
/// It is returned by [`Metadata::file_type`] method.
#[derive(Clone, Copy)]
pub struct FileType(u16);

impl Metadata {
    /// Returns the last access time of this metadata.
    ///
    /// The returned value corresponds to the `atime` field of `stat` on Unix
    /// platforms and the `ftLastAccessTime` field on Windows platforms.
    ///
    /// Note that not all platforms will keep this field update in a file's
    /// metadata, for example Windows has an option to disable updating this
    /// time when files are accessed and Linux similarly has `noatime`.
    ///
    /// # Errors
    ///
    /// This field might not be available on all platforms, and will return an
    /// `Err` on platforms where it is not available.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # osiris::block_on(async {
    /// use osiris::fs;
    ///
    /// let metadata = fs::metadata("foo.txt").await?;
    ///
    /// if let Ok(time) = metadata.accessed() {
    ///     println!("{time:?}");
    /// } else {
    ///     println!("Not supported on this platform");
    /// }
    /// # std::io::Result::Ok(()) }).unwrap();
    /// ```
    pub fn accessed(&self) -> std::io::Result<SystemTime> {
        #[cfg(target_family = "unix")]
        return Ok(system_time(self.statx.stx_atime));
        return Err(Error::from(io::ErrorKind::Unsupported));
    }

    /// Returns the creation time listed in this metadata.
    ///
    /// The returned value corresponds to the `btime` field of `statx` on
    /// Linux kernel starting from to 4.11, the `birthtime` field of `stat` on other
    /// Unix platforms, and the `ftCreationTime` field on Windows platforms.
    ///
    /// # Errors
    ///
    /// This field might not be available on all platforms, and will return an
    /// `Err` on platforms or filesystems where it is not available.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # osiris::block_on(async {
    /// use osiris::fs;
    /// let metadata = fs::metadata("foo.txt").await?;
    ///
    /// if let Ok(time) = metadata.created() {
    ///     println!("{time:?}");
    /// } else {
    ///     println!("Not supported on this platform or filesystem");
    /// }
    /// # std::io::Result::Ok(()) }).unwrap();
    /// ```
    pub fn created(&self) -> std::io::Result<SystemTime> {
        #[cfg(target_family = "unix")]
        return Ok(system_time(self.statx.stx_ctime));
        return Err(Error::from(io::ErrorKind::Unsupported));
    }

    /// Returns the last modification time listed in this metadata.
    ///
    /// The returned value corresponds to the `mtime` field of `stat` on Unix
    /// platforms and the `ftLastWriteTime` field on Windows platforms.
    ///
    /// # Errors
    ///
    /// This field might not be available on all platforms, and will return an
    /// `Err` on platforms where it is not available.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # osiris::block_on(async {
    /// use osiris::fs;
    ///
    /// let metadata = fs::metadata("Cargo.toml").await?;
    ///
    /// if let Ok(time) = metadata.modified() {
    ///     println!("{time:?}");
    /// } else {
    ///     println!("Not supported on this platform");
    /// }
    /// # std::io::Result::Ok(()) }).unwrap();
    /// ```
    pub fn modified(&self) -> io::Result<SystemTime> {
        #[cfg(target_family = "unix")]
        return Ok(system_time(self.statx.stx_mtime));
        return Err(Error::from(io::ErrorKind::Unsupported));
    }

    /// Returns `true` if this metadata is for a directory. The
    /// result is mutually exclusive to the result of
    /// [`Metadata::is_file`], and will be false for symlink metadata.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # osiris::block_on(async {
    /// use osiris::fs;
    ///
    /// let metadata = fs::metadata("./target").await?;
    /// assert!(!metadata.is_dir());
    /// # std::io::Result::Ok(()) }).unwrap();
    /// ```
    #[must_use]
    pub fn is_dir(&self) -> bool {
        self.file_type().is_dir()
    }

    /// Returns `true` if this metadata is for a regular file. The
    /// result is mutually exclusive to the result of
    /// [`Metadata::is_dir`], and will be false for symlink metadata.
    ///
    /// When the goal is simply to read from (or write to) the source, the most
    /// reliable way to test the source can be read (or written to) is to open
    /// it. Only using `is_file` can break workflows like `diff <( prog_a )` on
    /// a Unix-like system for example. See [`File::open`](crate::fs::File::open) or
    /// [`OpenOptions::open`](crate::fs::OpenOptions::open) for more information.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # osiris::block_on(async {
    /// use osiris::fs;
    ///
    /// let metadata = fs::metadata("Cargo.lock").await?;
    /// assert!(metadata.is_file());
    /// # std::io::Result::Ok(()) }).unwrap();
    /// ```
    #[must_use]
    pub fn is_file(&self) -> bool {
        self.file_type().is_file()
    }

    /// Returns the file type for this metadata.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use osiris::fs;
    ///
    /// #[osiris::main]
    /// async fn main() -> std::io::Result<()> {
    ///
    ///     let metadata = fs::metadata("foo.txt").await?;
    ///
    ///     println!("{:?}", metadata.file_type().is_file());
    ///     Ok(())
    /// }
    /// ```
    pub fn file_type(&self) -> FileType {
        FileType(self.statx.stx_mode)
    }

    /// Returns `true` if this metadata is for a symbolic link.
    #[must_use]
    pub fn is_symlink(&self) -> bool {
        self.file_type().is_symlink()
    }

    /// Returns the size of the file, in bytes, this metadata is for.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # osiris::block_on(async {
    /// use osiris::fs;
    ///
    /// let metadata = fs::metadata("Cargo.toml").await?;
    ///
    /// assert_ne!(0, metadata.len());
    ///
    ///
    /// # std::io::Result::Ok(()) }).unwrap();
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.statx.stx_size as usize
    }
}

impl FileType {
    /// Returns `true` if this metadata is for a directory. The
    /// result is mutually exclusive to the result of
    /// [`Metadata::is_file`], and will be false for symlink metadata.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # osiris::block_on(async {
    /// use osiris::fs;
    ///
    /// let metadata = fs::metadata("./target").await?;
    /// assert!(!metadata.file_type().is_dir());
    /// # std::io::Result::Ok(()) }).unwrap();
    /// ```
    #[must_use]
    pub fn is_dir(&self) -> bool {
        (self.0 as mode_t & S_IFMT) == S_IFDIR
    }

    /// Returns `true` if this metadata is for a regular file. The
    /// result is mutually exclusive to the result of
    /// [`Metadata::is_dir`], and will be false for symlink metadata.
    ///
    /// When the goal is simply to read from (or write to) the source, the most
    /// reliable way to test the source can be read (or written to) is to open
    /// it. Only using `is_file` can break workflows like `diff <( prog_a )` on
    /// a Unix-like system for example. See [`File::open`](crate::fs::File::open) or
    /// [`OpenOptions::open`](crate::fs::OpenOptions::open) for more information.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # osiris::block_on(async {
    /// use osiris::fs;
    ///
    /// let metadata = fs::metadata("Cargo.lock").await?;
    /// assert!(metadata.file_type().is_file());
    /// # std::io::Result::Ok(()) }).unwrap();
    /// ```
    #[must_use]
    pub fn is_file(&self) -> bool {
        (self.0 as mode_t & S_IFMT) == S_IFREG
    }

    /// Tests whether this file type represents a symbolic link.
    /// The result is mutually exclusive to the results of
    /// [`is_dir`] and [`is_file`]; only zero or one of these
    /// tests may pass.
    ///
    /// The underlying [`Metadata`] struct needs to be retrieved
    /// with the [`fs::symlink_metadata`] function and not the
    /// [`fs::metadata`] function. The [`fs::metadata`] function
    /// follows symbolic links, so [`is_symlink`] would always
    /// return `false` for the target file.
    ///
    /// [`fs::metadata`]: metadata
    /// [`fs::symlink_metadata`]: symlink_metadata
    /// [`is_dir`]: FileType::is_dir
    /// [`is_file`]: FileType::is_file
    /// [`is_symlink`]: FileType::is_symlink
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use osiris::fs;
    ///
    /// #[osiris::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let metadata = fs::metadata("foo.txt").await?;
    ///     let file_type = metadata.file_type();
    ///
    ///     assert_eq!(file_type.is_symlink(), false);
    ///     Ok(())
    /// }
    /// ```
    #[must_use]
    pub fn is_symlink(&self) -> bool {
        (self.0 as mode_t & S_IFMT) == S_IFLNK
    }

    /// Returns `true` if this file type is a fifo.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use osiris::fs;
    /// use std::io;
    ///
    /// #[osiris::main]
    /// async fn main() -> io::Result<()> {
    ///     let meta = fs::metadata("fifo_file").await?;
    ///     let file_type = meta.file_type();
    ///     assert!(file_type.is_fifo());
    ///     Ok(())
    /// }
    /// ```
    pub fn is_fifo(&self) -> bool {
        (self.0 as mode_t & libc::S_IFIFO) == libc::S_IFIFO
    }
}

#[cfg(target_os = "linux")]
fn system_time(time: statx_timestamp) -> SystemTime {
    let secs = Duration::from_secs(time.tv_sec as _);
    let nanos = Duration::from_nanos(time.tv_nsec as _);
    SystemTime::UNIX_EPOCH + secs + nanos
}
