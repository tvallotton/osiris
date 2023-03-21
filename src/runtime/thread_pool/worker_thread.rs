use super::Callback;
use core::panic;
use std::any::Any;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc::TrySendError::Full;
use std::sync::mpsc::{sync_channel as channel, Receiver, SyncSender};
use std::thread::{spawn, Thread};
use std::time::{Duration, Instant};

// A spawn blocking worker thread
pub struct WorkerThread {
    sender: SyncSender<Callback>,
    receiver: Receiver<(Result<Box<dyn Any + Send>, Box<dyn Any + Send>>, Duration)>,
}

impl WorkerThread {
    // spawns a worker thread
    pub fn spawn() -> Self {
        let (tx_work, rx_work) = channel::<Callback>(0);
        let (tx_result, rx_result) = channel(0);

        let worker = WorkerThread {
            sender: tx_work,
            receiver: rx_result,
        };

        spawn(move || {
            while let Ok(work) = rx_work.recv() {
                let time = Instant::now();
                let res = catch_unwind(AssertUnwindSafe(work));
                // should we unwrap this?
                tx_result.send((res, time.elapsed())).unwrap();
            }
        });
        worker
    }

    pub fn try_send(self, f: Callback) -> Option<Callback> {
        let Full(f) = self.sender.try_send(f).err()? else { 
            panic!("unexpected dead worker thread.") 
        };
        Some(f)
    }
}
