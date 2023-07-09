use std::{
    collections::HashMap,
    io::{self, Error, Result},
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    // ptr::{null},
    task::Waker,
};

use crate::runtime::Config;

use self::event::id;
mod event;
pub mod op;

/// KQueue driver
pub(crate) struct Driver {
    /// we use this to generate new ids on demand
    event_id: usize,
    /// the kqueue file descriptor
    fd: OwnedFd,
    /// the stack of events we are interested in
    queue: Vec<libc::kevent>,

    wakers: HashMap<(usize, i16), Waker>,
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
            wakers: HashMap::with_capacity(config.queue_entries as usize),
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
        // println!("submitting and wait");
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
        let len        = unsafe {
            libc::kevent(kq, changelist, nchanges, eventlist, nevents, timeout)
        };
        if len < 0 {
             Err(Error::last_os_error())
        } else {
            unsafe{ self.queue.set_len(len as usize) };
            Ok(())
        }
    }

    /// wakes up any tasks listening for IO events.
    pub fn wake_tasks(&mut self) {
        for event in &self.queue {
            let Some(waker) = self.wakers.remove(&id(*event)) else {
                continue;
            };
            println!("waking");
            waker.wake();
        }
        self.queue.clear();
    }

    pub fn push(&mut self, event: libc::kevent, waker: Waker) -> Result<()> {
        if self.queue.len() * 2 >= self.queue.capacity() {
            self.submit_and_yield()?;
            self.wake_tasks();
        }
        let k = (event.ident, event.filter);
        self.wakers.insert(k, waker);
        self.queue.push(event);
        Ok(())
    }

    #[inline]
    pub fn event_id(&mut self) -> usize {
        self.event_id += 1;
        self.event_id
    }
}
