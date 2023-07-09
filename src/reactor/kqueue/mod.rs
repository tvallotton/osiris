use std::{
    collections::HashMap,
    io::{self, Error, Result},
    ptr::{null},
    task::{Waker}
};

use crate::runtime::Config;
mod event;
mod op;



pub(crate) struct Driver {
    event_id: usize,
    fd: i32,
    queue: Vec<libc::kevent>,
    wakers: HashMap<u64, Waker>,
}

impl Driver {
    pub fn new(config: Config) -> io::Result<Driver> {
        let fd = unsafe { libc::kqueue() };
        if fd < 0 {
            return Err(Error::last_os_error());
        }
        let driver = Driver {
            fd,
            event_id: 0,
            queue: Vec::with_capacity(config.queue_entries as usize * 2 ),
            wakers: HashMap::with_capacity(config.queue_entries as usize),
        };
        Ok(driver)
    }

    
    pub fn submit_and_yield(&mut self) -> io::Result<()> {
        self.submit(&libc::timespec {
            tv_nsec: 0, 
            tv_sec: 0
        })
    }

    
    pub fn submit_and_wait(&mut self) -> io::Result<()> {
        self.submit(null())
    }

    #[rustfmt::skip]
    fn submit(&mut self, timeout: *const libc::timespec) -> io::Result<()> {
        let kq         = self.fd;
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
            let id = event.udata as u64; 
            let Some(waker) = self.wakers.remove(&id) else {
                continue;
            }; 
            waker.wake();
        };
        self.queue.clear();
    }

    pub fn push(&mut self, mut event: libc::kevent) -> Result<usize> {
        if self.queue.len() * 2 >= self.queue.capacity() {
            self.submit_and_yield()?; 
        }
        let id = self.event_id();
        event.udata = id as _; 
        self.queue.push(event);
        Ok(id)
    }

    #[inline]
    pub fn event_id(&mut self) -> usize {
        self.event_id += 1;
        self.event_id
    }

}
