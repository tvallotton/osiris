

//! Implementation summary: 
//! 
//! | Routine          | Complexity| Ideal      | Function calls                          |
//! |------------------|-----------|------------|-----------------------------------------|
//! | push             | O(1)      | O(1)       | 3 * Vec::push                           |
//! | cancellation     | O(n)      | O(1)       | n * Vec::index + 2  * Vec::swap_remove  |
//! | wake_tasks       | O(n)      | O(n)       | n * Vec::index + 2m * Vec::swap_remove  |
//! 
//! Where n`` is the total number of io events and `m`` is the actual number of
//! io events to be woken
//! 


use std::io;
use std::task::Waker;
use std::time::Duration;

use crate::runtime::Config;
use crate::utils::syscall;

pub use libc::pollfd as Event;

pub mod op;


pub(crate) struct Driver {
    event_id: u64,
    wakers: Vec<(u64, Waker)>,
    fds: Vec<Event>,
    to_wake: i32
}


impl Driver {
    pub fn new(config: Config) -> io::Result<Self> {
        let driver = Driver {
            event_id: 0, 
            wakers: Vec::with_capacity(config.queue_entries as usize * 2), 
            fds: Vec::with_capacity(config.queue_entries as usize * 2),
            to_wake: 0,
        };

        Ok(driver)
    }

    #[inline]
    pub fn event_id(&mut self) -> u64 {
        self.event_id += 1;
        self.event_id
    }

    pub fn submit_and_yield(&mut self) -> io::Result<()> {
        self.submit(Duration::ZERO)
    }

    pub fn submit_and_wait(&mut self) -> io::Result<()> {
        let timeout = Duration::from_secs(60); 
        self.submit(timeout)
    }

    #[rustfmt::skip]
    fn submit(&mut self, timeout: Duration) -> io::Result<()> {
        let timeout = timeout.as_millis() as i32; 
        let len = self.fds.len() as u64;
        let fds = self.fds.as_mut_ptr();
        self.to_wake = syscall!(poll, fds, len, timeout)?;
        Ok(())
    }

    pub fn wake_tasks(&mut self) {
        assert!(self.fds.len() == self.wakers.len());
        let mut i = 0;
        while i < self.fds.len() {
            let pollfd = self.fds[i];
            if pollfd.revents == 0 {
                i += 1;
                continue
            }
            self.fds.swap_remove(i); 
            let (_, waker) = self.wakers.swap_remove(i); 
            waker.wake(); 

            self.to_wake -= 1;
            if self.to_wake <= 0 {
                return
            }
        }
    }

    pub fn remove_waker(&mut self, id: u64) {
        for i in 0..self.wakers.len() {
            let (event_id, _) = &self.wakers[i];
            if *event_id != id {
                continue;
            }
            self.wakers.swap_remove(i);
            self.fds.swap_remove(i);
            break;
        }

    }

    pub fn push(&mut self, pollfd: Event, waker: Waker) -> u64 {
        let id = self.event_id(); 
        self.fds.push(pollfd); 
        self.wakers.push((id, waker)); 
        id
    }

    

}
