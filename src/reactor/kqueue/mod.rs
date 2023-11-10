use std::{
    collections::{hash_map::Entry, HashMap},
    io::{self, Error, Result},
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    sync::Arc,
    // ptr::{null},
    task::{Wake, Waker},
};

use crate::runtime::Config;
use crate::utils::syscall;

use self::event::id;
pub use libc::kevent as Event;

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
        let len        = syscall!(kevent, kq, changelist, nchanges, eventlist, nevents, timeout)?;
        unsafe{ self.queue.set_len(len as usize) };
        Ok(())
    }

    /// wakes up any tasks listening for IO events.
    pub fn wake_tasks(&mut self) {
        for event in &self.queue {
            let Some(waker) = self.wakers.remove(&id(*event)) else {
                continue;
            };
            waker.wake();
        }
        self.queue.clear();
    }

    pub fn remove_waker(&mut self, waker: u64) {
        // self.wakers.remove(waker);
        todo!()
    }

    pub fn push(&mut self, event: libc::kevent, waker: Waker) -> Result<()> {
        if self.queue.len() * 2 >= self.queue.capacity() {
            self.submit_and_yield()?;
            self.wake_tasks();
        }
        let k = (event.ident, event.filter);

        self.wakers.insert(k, waker);

        match self.wakers.entry(k) {
            Entry::Vacant(k) => {
                k.insert(waker);
            }
            Entry::Occupied(mut k) => {
                k.insert(join(waker, k.get().clone()));
            }
        }

        self.queue.push(event);
        Ok(())
    }

    #[inline]
    pub fn event_id(&mut self) -> usize {
        self.event_id += 1;
        self.event_id
    }
}

pub fn join(w1: Waker, w2: Waker) -> Waker {
    struct JoinWaker(Waker, Waker);
    impl Wake for JoinWaker {
        fn wake(self: Arc<Self>) {
            self.0.wake();
            self.1.wake();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.wake_by_ref();
            self.1.wake_by_ref();
        }
    }
    Arc::new(JoinWaker(w1, w2)).into()
}
