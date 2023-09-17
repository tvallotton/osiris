use std::{
    future::poll_fn,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    task::Poll,
};

use crossbeam_channel::{unbounded, Receiver, Sender};

use self::work::Work;
use crate::{
    runtime::thread_pool::work::work,
    task::{self, JoinHandle},
    time::timeout,
};
use std::thread;

use super::{config::ThreadPoolConfig, Config};

mod work;

pub(crate) struct ThreadPool {
    config: ThreadPoolConfig,
    sender: Sender<Arc<dyn Work>>,
    receiver: Receiver<Arc<dyn Work>>,
    workers: Arc<AtomicU32>,
}

impl ThreadPool {
    pub fn new(config: Config) -> Self {
        let (sender, receiver) = unbounded();
        let workers = Arc::new(AtomicU32::new(0));
        let config = config.thread_pool;
        ThreadPool {
            config,
            sender,
            receiver,
            workers,
        }
    }

    pub fn spawn_blocking<T, F>(&'static self, f: F) -> JoinHandle<T>
    where
        T: Send + Sync + 'static,
        F: FnOnce() -> T + Send + Sync + 'static,
    {
        task::spawn(async move {
            let waker = poll_fn(|cx| Poll::Ready(cx.waker().clone())).await;
            let work = work(f, waker);
            self.sender.send(work.clone()).unwrap();
            self.ensure_workers();
            let dur = self.config.wait_timeout;
            loop {
                match timeout(dur, resolve::<T>(&*work)).await {
                    Err(_) => self.spawn_worker(),
                    Ok(t) => return t,
                }
            }
        })
    }

    fn ensure_workers(&self) {
        let workers = self.workers.load(Ordering::Acquire);
        if workers < 1 {
            self.spawn_worker()
        }
    }

    fn spawn_worker(&self) {
        let workers = self.workers.load(Ordering::Acquire);
        if workers < self.config.max_workers {
            self.spawn_worker_unchecked()
        }
    }

    fn spawn_worker_unchecked(&self) {
        let workers = self.workers.clone();
        workers.fetch_add(1, Ordering::Release);
        let receiver = self.receiver.clone();
        let timeout = self.config.idle_timeout;
        thread::spawn(move || loop {
            let Ok(work) = receiver.recv_timeout(timeout) else {
                workers.fetch_sub(1, Ordering::Release);
                break;
            };
            work.block();
        });
    }
}

async fn resolve<T>(work: &dyn Work) -> T
where
    T: 'static,
{
    poll_fn(move |_| {
        let mut out = Poll::Pending;
        work.take(&mut out);
        out
    })
    .await
}
