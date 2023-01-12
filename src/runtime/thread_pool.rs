#[allow(warnings)]
use std::collections::VecDeque;
use std::fmt::Debug;
use std::intrinsics::breakpoint;
use std::ops::{Deref, DerefMut};
use std::panic::catch_unwind;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Mutex, MutexGuard};
use std::thread::spawn;

use super::Config;

type Callback = Box<dyn FnOnce() + Send>;

pub struct ThreadPool(Mutex<Inner>);

pub struct Inner {
    config: Config,
    queue: VecDeque<Callback>,
    threads: VecDeque<Thread>,
}

pub struct Thread {
    is_idle: bool,
    sender: Sender<Callback>,
    receiver: Receiver<Result<(), Box<dyn Debug + Send>>>,
}

impl Inner {
    fn poll(&mut self) {
        while let Some(val) = self.queue.pop_front() {
            for thread in self.threads {
                if thread.is_idle {
                    thread.sender.send(val);
                    continue;
                }
            }
            break;
        }
    }
    /// Pushes a new thread onto the runtime
    fn push(&self) {
        let (tx_work, rx_work) = channel();
        let (tx_res, rx_res) = channel();
        spawn(move || {
            while let Ok(work) = rx_work.recv() {
                let res = catch_unwind(move || work());
                tx_res(res);
            }
        });

        let thread = Thread {
            is_idle: true,
            sender: tx_wrk,
            receiver: rx_res,
        };
        self.threads.push_front(thread);
    }

    fn pop(&mut self) {
        while let Some(t) = self.threads.pop_back() {
            if t.is_idle {
                break;
            }

            self.threads.push_front(t);
        }
    }
}

impl ThreadPool {
    fn inner(&self) -> impl DerefMut<Target = Inner> + '_ {
        self.0.lock().unwrap()
    }

    ///
    pub fn poll(&mut self) {
        self.inner().poll()
    }

    pub fn spawn(&self, f: impl FnOnce() + Send) {
        let mut inner = self.inner();
        inner.queue.push_back(Box::new(f));
        inner.poll();
    }

    // pub fn init_shared(&self) -> ThreadPool {}

    pub fn push(&mut self, f: impl FnOnce() + Send) {
        self.queue.push_back(Box::new(f));
    }
}
