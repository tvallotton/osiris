use crate::runtime::{Config, Mode};
use io_uring::squeue::Entry;
use io_uring::IoUring;

const DEFAULT_WAKERS: usize = 2048;

#[non_exhaustive]
pub(crate) enum Pollster {
    IoUring(IoUring),
}

impl Pollster {
    pub fn new(config: Config) -> std::io::Result<Pollster> {
        let mut builder = IoUring::builder();
        if let Mode::Polling { idle_timeout } = config.mode {
            builder.setup_sqpoll(idle_timeout);
        }
        let io_uring = builder.build(config.io_uring_entries)?;
        Ok(Pollster::IoUring(io_uring))
    }
    /// # Safety
    ///
    /// Developers must ensure that parameters of the entry (such as buffer) are valid and will
    /// be valid for the entire duration of the operation, otherwise it may cause memory problems.
    pub unsafe fn sumit_io(&mut self, entry: Entry) -> std::io::Result<()> {
        match self {
            Pollster::IoUring(ring) => {
                if ring.submission().is_full() {
                    ring.submit()?;
                    // SAFETY:
                    // the validity of the entry is upheld by the caller.
                    unsafe { ring.submission().push(&entry).ok() };
                }
                Ok(())
            }
        }
    }

    pub fn woken(&mut self) -> impl Iterator<Item = u64> + '_ {
        match self {
            Pollster::IoUring(ring) => ring.completion().into_iter().map(|entry| entry.user_data()),
        }
    }
}
