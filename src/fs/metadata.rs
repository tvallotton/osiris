use crate::shared_driver::submit;
use io_uring::opcode::Statx;
use io_uring::types::Fd;
use libc::{statx, S_IFDIR, S_IFLNK, S_IFMT, S_IFREG};
use std::ffi::CString;
use std::io;
use std::mem::MaybeUninit;
use std::os::unix::prelude::OsStrExt;
use std::path::Path;
use std::time::{Duration, SystemTime};

pub async fn metadata(path: impl AsRef<Path>) -> std::io::Result<Metadata> {
    _metadata(path.as_ref()).await
}

async fn _metadata(path: &Path) -> std::io::Result<Metadata> {
    let path = CString::new(path.as_os_str().as_bytes()).unwrap();
    let mut statxbuf = Box::new(MaybeUninit::<statx>::uninit());
    let sqe = Statx::new(
        Fd(libc::AT_FDCWD),
        path.as_ptr().cast(),
        statxbuf.as_mut_ptr().cast(),
    )
    .mask(libc::STATX_ALL as _)
    .build();
    let (cqe, (statx, _)) = unsafe { submit(sqe, (statxbuf, path)).await };
    cqe?;
    // Safety: initialized by io-uring
    let statx = unsafe { MaybeUninit::assume_init(*statx) };
    Ok(Metadata { statx })
}

pub struct Metadata {
    #[cfg(target_os = "linux")]
    pub(crate) statx: statx,
}

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
    /// use osiris::fs;
    ///
    /// #[osiris::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let metadata = fs::metadata("foo.txt").await?;
    ///
    ///     if let Ok(time) = metadata.accessed() {
    ///         println!("{time:?}");
    ///     } else {
    ///         println!("Not supported on this platform");
    ///     }
    ///     Ok(())
    /// }
    /// ```
    fn accessed(&self) -> std::io::Result<SystemTime> {
        Ok(system_time(self.statx.stx_atime))
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
    /// use osiris::fs;
    ///
    /// #[osiris::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let metadata = fs::metadata("foo.txt").await?;
    ///
    ///     if let Ok(time) = metadata.created() {
    ///         println!("{time:?}");
    ///     } else {
    ///         println!("Not supported on this platform or filesystem");
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn created(&self) -> std::io::Result<SystemTime> {
        Ok(system_time(self.statx.stx_ctime))
    }

    /// Returns `true` if this metadata is for a directory. The
    /// result is mutually exclusive to the result of
    /// [`Metadata::is_file`], and will be false for symlink metadata
    /// obtained from [`symlink_metadata`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use osiris::fs;
    ///
    /// #[osiris::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let metadata = fs::metadata("./target")?;
    ///
    ///     assert!(!metadata.is_dir());
    ///     Ok(())
    /// }
    /// ```
    #[must_use]
    pub fn is_dir(&self) -> bool {
        (self.statx.stx_mode as u32 & S_IFMT) == S_IFDIR
    }

    /// Returns `true` if this metadata is for a regular file. The
    /// result is mutually exclusive to the result of
    /// [`Metadata::is_dir`], and will be false for symlink metadata
    /// obtained from [`symlink_metadata`].
    ///
    /// When the goal is simply to read from (or write to) the source, the most
    /// reliable way to test the source can be read (or written to) is to open
    /// it. Only using `is_file` can break workflows like `diff <( prog_a )` on
    /// a Unix-like system for example. See [`File::open`] or
    /// [`OpenOptions::open`] for more information.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use osiris::fs;
    ///
    /// #[osiris::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let metadata = fs::metadata("Cargo.lock").await?;
    ///
    ///     assert!(metadata.is_file());
    ///     Ok(())
    /// }
    /// ```
    #[must_use]
    pub fn is_file(&self) -> bool {
        (self.statx.stx_mode as u32 & S_IFMT) == S_IFREG
    }

    /// Returns `true` if this metadata is for a symbolic link.
    #[must_use]
    pub fn is_symlink(&self) -> bool {
        (self.statx.stx_mode as u32 & S_IFMT) == S_IFLNK
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
    /// use osiris::fs;
    ///
    /// #[osiris::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let metadata = fs::metadata("Cargo.toml").await?;
    ///
    ///     if let Ok(time) = metadata.modified() {
    ///         println!("{time:?}");
    ///     } else {
    ///         println!("Not supported on this platform");
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn modified(&self) -> io::Result<SystemTime> {
        Ok(system_time(self.statx.stx_mtime))
    }

    /// Returns the size of the file, in bytes, this metadata is for.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use osiris::fs;
    ///
    /// #[osiris::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let metadata = fs::metadata("Cargo.toml").await?;
    ///
    ///     assert_ne!(0, metadata.len());
    ///     Ok(())
    /// }
    /// ```
    #[must_use]
    pub fn len(&self) -> u64 {
        self.statx.stx_size
    }
}

fn system_time(time: libc::statx_timestamp) -> SystemTime {
    let secs = Duration::from_secs(time.tv_sec as _);
    let nanos = Duration::from_nanos(time.tv_nsec as _);
    SystemTime::UNIX_EPOCH + secs + nanos
}
