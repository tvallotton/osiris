use std::any::Any;

use std::cmp::Ordering;
use std::collections::HashMap;
#[allow(warnings)]
use std::collections::VecDeque;
use std::future::{poll_fn, Future};
use std::iter::Once;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::Ordering::AcqRel;
use std::sync::atomic::{AtomicI32, AtomicI64, AtomicU32};
use std::task::{Poll, Waker};
use std::time::{Instant, SystemTime};

use self::worker_thread::WorkerThread;

use super::config::ThreadPoolConfig;
use super::current_unwrap;

use std::panic::{catch_unwind, AssertUnwindSafe};

pub use handle::ThreadPoolHandle;
use stats::Stats;
use std::sync::mpsc::{sync_channel as channel, Receiver, SyncSender};
use std::sync::{Mutex, PoisonError, RwLock};
use std::thread::spawn;
use work::{Work, WorkResult};
use worker_thread::Worker;

mod handle;
mod stats;
mod work;
mod worker_thread;


/// # Spawn blocking thread pool
pub struct ThreadPool {
    config: ThreadPoolConfig,
    stats: Stats,
    id: u32,
    results: HashMap<u32, WorkResult>,
    // the queued work to be performed
    queue: VecDeque<Work>,
    // a queue with the worker threads
    workers: VecDeque<Worker>,
}

pub fn lock_thread_pool() -> impl DerefMut<Target = ThreadPool> {
    static GLOBAL_THREAD_POOL : Mutex<ThreadPool> = Mutex::new(ThreadPool {
        id: 0, 
        config: ThreadPoolConfig::default(),
        stats: Stats::new(),
        results: Default::default(), 
        queue: Default::default(), 
        workers: Default::default(), 
    });
    GLOBAL_THREAD_POOL.lock().unwrap_or_else(ignore_poison)
}

pub fn spawn_blocking<F, T>(f: F) -> impl Future<Output = T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let rt = current_unwrap("spawn_blocking");
    let handle = None;
    poll_fn(|cx| {
        let Some(handle) = handle else {
            handle = rt.pool
            return Poll::Pending
        };

        let Some(result) = rt.pool.try_get_result(handle.id) else {
            rt.pool.wakers.lock().unwrap_or_else(ignore_poison).insert(handle.id, cx.waker().clone());
            return Poll::Pending
        };
        match result {
            WorkResult::Ok(v) => Poll::Ready(v),
            WorkResult::Panic(e) => panic!(e),
        }
    })
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

    pub fn push_work<T, F>(&self, f: F, waker: Waker) -> Handle
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let id = self.id.fetch_add(1, AcqRel);
        self.queue
            .lock()
            .unwrap_or_else(ignore_poison)
            .push_back(Work::new(id, f));
        self.wakers
            .lock()
            .unwrap_or_else(ignore_poison)
            .insert(id, waker);

        Handle { pool: self, id }
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

fn ignore_poison<T>(e: PoisonError<T>) -> T {
    e.into_inner()
}
