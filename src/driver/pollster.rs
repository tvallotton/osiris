use crate::hasher::NoopHasher;
use crate::runtime::{Config, Mode};
use io_uring::IoUring;
use io_uring::squeue::Entry;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

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


    pub unsafe fn sumit_io(&mut self,entry: Entry) {
        match self {
            Pollster::IoUring(ring) => {



            }
        }
    }
}
