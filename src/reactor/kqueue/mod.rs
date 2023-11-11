use crate::runtime::Config;
use crate::utils::syscall;

use slab::Slab;
use std::io::{self, Error, Result};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::task::Waker;

pub use crate::reactor::nonblocking::*;
pub use libc::kevent as Event;

pub mod op;

/// KQueue driver
pub(crate) struct Driver {
    /// we use this to generate new ids on demand
    event_id: usize,
    /// the kqueue file descriptor
    fd: OwnedFd,
    /// the stack of events we are interested in
    queue: Vec<libc::kevent>,

    wakers: Slab<Waker>,
}

impl Driver {
    pub fn new(config: Config) -> io::Result<Driver> {
        let fd = unsafe { libc::kqueue() };
        if fd < 0 {
            return Err(Error::last_os_error());
        }
        let driver = Driver {
            fd: unsafe { OwnedFd::from_raw_fd(fd) },
            event_id: 0,
            queue: Vec::with_capacity(config.queue_entries as usize * 2),
            wakers: Slab::with_capacity(config.queue_entries as usize),
        };
        Ok(driver)
    }

    pub fn submit_and_yield(&mut self) -> io::Result<()> {
        self.submit(&libc::timespec {
            tv_nsec: 0,
            tv_sec: 0,
        })
    }

    pub fn submit_and_wait(&mut self) -> io::Result<()> {
        self.submit(&libc::timespec {
            tv_nsec: 0,
            tv_sec: 60,
        })
    }

    #[rustfmt::skip]
    fn submit(&mut self, timeout: *const libc::timespec) -> io::Result<()> {
        let kq         = self.fd.as_raw_fd();
        let changelist = self.queue.as_ptr();
        let eventlist  = self.queue.as_mut_ptr();
        let nevents    = self.queue.capacity() as i32;
        let nchanges   = self.queue.len() as i32;
        let len        = syscall!(kevent, kq, changelist, nchanges, eventlist, nevents, timeout)?;
        unsafe { self.queue.set_len(len as usize) };
        self.wake_tasks();
        Ok(())
    }

    pub fn remove_waker(&mut self, waker: u64) {
        self.wakers.try_remove(waker as usize);
    }

    pub fn push(&mut self, mut event: libc::kevent, waker: Waker) -> Result<u64> {
        if self.queue.len() * 2 >= self.queue.capacity() {
            self.submit_and_yield()?;
        }
        let id = self.wakers.insert(waker);
        event.udata = id as _;
        self.queue.push(event);
        Ok(id as u64)
    }

    fn wake_tasks(&mut self) {
        for event in &self.queue {
            let option = self.wakers.get(event.udata as usize);
            let Some(waker) = option else {
                continue;
            };
            waker.wake_by_ref();
        }
        self.queue.clear();
    }
}
