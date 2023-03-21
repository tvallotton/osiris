use std::any::Any;
#[allow(warnings)]
use std::collections::VecDeque;
use std::time::{Instant, SystemTime};

use self::worker_thread::WorkerThread;

use super::config::ThreadPoolConfig;

use std::panic::{catch_unwind, AssertUnwindSafe};

use stats::Stats;
use std::sync::mpsc::TrySendError::Full;
use std::sync::mpsc::{sync_channel as channel, Receiver, SyncSender};
use std::sync::{Mutex, RwLock};
use std::thread::spawn;
use worker_thread::Thread;

mod handle;
mod stats;
mod worker_thread;

const POISONED_MUTEX_ERR: &str = "unexpected spawn_blocking poisoned mutex.";
const WORKER_THREAD_ERR: &str = "unexpected dead spawn_blocking worker thread.";

type Callback = Box<dyn FnOnce() -> Box<dyn Any + Send> + Send>;

/// # Spawn blocking thread pool
pub struct ThreadPool {
    config: ThreadPoolConfig,
    stats: Stats,
    queue: Mutex<VecDeque<Callback>>,
    workers: RwLock<VecDeque<WorkerThread>>,
}

impl ThreadPool {
    /// polls to send queued values to the worker threads.
    pub fn poll(&mut self) {
        self.try_poll().expect(POISONED_MUTEX_ERR);
    }

    pub fn try_poll(&mut self) -> Option<()> {
        'main: loop {
            let mut queue = self.queue.lock().ok()?;
            let Some(mut work) = queue.pop_front() else {
                break Some(());
            };
            drop(queue);
            for worker in self.workers.read().ok()?.iter() {
                let Some(w) = worker.try_send(work) else {
                    continue 'main;
                };
                work = w;
            }
            // no workers were available
            self.queue.lock().ok()?.push_front(work);
            break Some(());
        }
    }
    /// computes the number of servers required using the
    /// queuing rule of thumb:
    /// ```
    /// s > N*r/T
    /// ```
    /// where `r` is the service time, and N/T is the arival rate.
    fn n_servers(&self) -> usize {
        todo!()
    }

    pub fn push_work<T, F>(&self, f: F)
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        self.queue
            .lock()
            .expect(POISONED_MUTEX_ERR)
            .push_back(Box::new(move || Box::new(f())));
    }

    pub fn spawn_worker(&self) {
        let (tx_work, rx_work) = channel::<Callback>(0);
        let (tx_result, rx_result) = channel(0);

        let thread = Thread {
            sender: tx_work,
            receiver: rx_result,
        };

        spawn(move || {
            while let Ok(work) = rx_work.recv() {
                let res = catch_unwind(AssertUnwindSafe(work));
                // should we unwrap this?
                tx_result.send(res).unwrap();
            }
        });

        self.workers
            .write()
            .expect(POISONED_MUTEX_ERR)
            .push_back(thread);
    }
}
