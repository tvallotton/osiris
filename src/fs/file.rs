#![allow(clippy::missing_errors_doc, warnings)]

use io_uring::types::Fd;

use crate::buf::{deref, IoBuf};
use crate::detach;
use crate::shared_driver::submit;
use crate::sync::Mutex;
use std::io;
use std::path::Path;
use std::sync::MutexGuard;

use super::OpenOptions;

pub struct File {
    pub(crate) fd: Option<i32>,
}

impl Drop for File {
    fn drop(&mut self) {
        let Some(fd) = self.fd.take() else { return; };
        detach(async move { File { fd: Some(fd) }.close().await });
    }
}

impl File {
    /// Attempts to open a file in read-only mode.
    ///
    /// See the [`OpenOptions::open`] method for more details.
    ///
    /// # Errors
    ///
    /// This function will return an error if `path` does not already exist.
    /// Other errors may also be returned according to [`OpenOptions::open`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use osiris::fs::File;
    ///
    /// #[osiris::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let mut f = File::open("foo.txt")?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn open<P: AsRef<Path>>(path: P) -> io::Result<File> {
        OpenOptions::new().read(true).open(path.as_ref()).await
    }
    /// Opens a file in write-only mode.
    ///
    /// This function will create a file if it does not exist,
    /// and will truncate it if it does.
    ///
    /// Depending on the platform, this function may fail if the
    /// full directory path does not exist.
    ///
    /// See the [`OpenOptions::open`] function for more details.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use osiris::fs::File;
    ///
    /// #[osiris::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let mut f = File::create("foo.txt")?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn create<P: AsRef<Path>>(path: P) -> io::Result<File> {
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path.as_ref())
            .await
    }
    /// Creates a new file in read-write mode; error if the file exists.
    ///
    /// This function will create a file if it does not exist, or return an error if it does. This
    /// way, if the call succeeds, the file returned is guaranteed to be new.
    ///
    /// This option is useful because it is atomic. Otherwise between checking whether a file
    /// exists and creating a new one, the file may have been created by another process (a TOCTOU
    /// race condition / attack).
    ///
    /// This can also be written using
    /// `File::options().read(true).write(true).create_new(true).open(...)`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use osiris::fs::File;
    ///
    /// #[osiris::main]
    /// async fn main() -> std::io::Result<()> {
    ///     let mut f = File::create_new("foo.txt")?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn create_new<P: AsRef<Path>>(path: P) -> io::Result<File> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path.as_ref())
            .await
    }

    /// Returns a new `OpenOptions` object.
    ///
    /// This function returns a new `OpenOptions` object that you can use to
    /// open or create a file with specific options if `open()` or `create()`
    /// are not appropriate.
    ///
    /// It is equivalent to `OpenOptions::new()`, but allows you to write more
    /// readable code. Instead of
    /// `OpenOptions::new().append(true).open("example.log")`,
    /// you can write `File::options().append(true).open("example.log")`. This
    /// also avoids the need to import `OpenOptions`.
    ///
    /// See the [`OpenOptions::new`] function for more details.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use osiris::fs::File;
    ///
    /// #[osiris::main]
    /// fn main() -> std::io::Result<()> {
    ///     let mut f = File::options().append(true).open("example.log")?;
    ///     Ok(())
    /// }
    /// ```
    #[must_use]
    pub fn options() -> OpenOptions {
        OpenOptions::new()
    }

    /// Closes the file.
    ///
    /// The method completes once the close operation has completed,
    /// guaranteeing that resources associated with the file have been released.
    ///
    /// If `close` is not called before dropping the file, the file is closed in
    /// the background, but there is no guarantee as to **when** the close
    /// operation will complete.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use osiris::fs::File;
    ///
    /// #[osiris::main]
    /// async fn main() -> Result<(), std::io::Error> {
    ///     // open the file
    ///     let f = File::open("foo.txt").await?;
    ///     // close the file
    ///     f.close().await?;
    ///
    ///     Ok(())
    /// }
    /// ```

    pub async fn close(mut self) -> io::Result<()> {
        use crate::shared_driver::submit;
        use io_uring::types::Fd;
        let fd = self.fd.unwrap();
        let entry = io_uring::opcode::Close::new(Fd(fd)).build();
        let (entry, _) = unsafe { submit(entry, ()).await };
        entry?;
        Ok(())
    }

    /// Write a buffer into this file at the specified offset, returning how
    /// many bytes were written.
    ///
    /// This function will attempt to write the entire contents of `buf`, but
    /// the entire write may not succeed, or the write may also generate an
    /// error. The bytes will be written starting at the specified offset.
    ///
    /// # Return
    ///
    /// The method returns the operation result and the same buffer value passed
    /// in as an argument. A return value of `0` typically means that the
    /// underlying file is no longer able to accept bytes and will likely not be
    /// able to in the future as well, or that the buffer provided is empty.
    ///
    /// # Errors
    ///
    /// Each call to `write` may generate an I/O error indicating that the
    /// operation could not be completed. If an error is returned then no bytes
    /// in the buffer were written to this writer.
    ///
    /// It is **not** considered an error if the entire buffer could not be
    /// written to this writer.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use osiris::fs::File;
    ///
    /// #[osiris::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let file = File::create("foo.txt").await?;
    ///
    ///     // Writes some prefix of the byte string, not necessarily all of it.
    ///     let (res, _) = file.write_at(&b"some bytes"[..], 0).await;
    ///     let n = res?;
    ///
    ///     println!("wrote {} bytes", n);
    ///
    ///     // Close the file
    ///     file.close().await?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// [`Ok(n)`]: Ok
    pub async fn write_at<T: IoBuf>(&mut self, buf: T, pos: usize) -> (io::Result<usize>, T) {
        use io_uring::opcode::Write;

        let fd = self.fd.unwrap();
        let len = buf.bytes_init();
        let buf = buf.slice(pos..len);

        let entry = Write::new(Fd(fd), buf.stable_ptr(), buf.len() as _).build();

        match unsafe { submit(entry, buf).await } {
            (Err(err), buf) => {
                return (Err(err), buf.into_inner());
            }
            (Ok(entry), buf) => (Ok(entry.result() as _), buf.into_inner()),
        }
    }
}
