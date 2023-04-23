use super::{Work, WorkResult};

use std::sync::mpsc::TrySendError::Disconnected;
use std::sync::mpsc::{sync_channel as channel, Receiver, SyncSender};
use std::sync::{PoisonError, RwLock};
use std::thread::spawn;
use std::time::Instant;

// A spawn blocking worker thread
pub struct Worker {
    sender: RwLock<Option<SyncSender<Work>>>,
    receiver: Receiver<WorkResult>,
}

impl Worker {
    // spawns a worker thread
    pub fn spawn() -> Self {
        let (work_sender, work_recv) = channel(0);
        let (result_sender, result_recv) = channel(0);

        let worker = Worker {
            sender: RwLock::new(Some(work_sender)),
            receiver: result_recv,
        };

        spawn(move || {
            while let Ok(work) = work_recv.recv() {
                let time = Instant::now();
                // should we unwrap this?
                result_sender.send(work.execute()).unwrap();
            }
        });
        worker
    }

    pub fn try_send(&self, f: Work) -> Result<(), Work> {
        let guard = self.sender.read().unwrap_or_else(ignore_poison);
        let Some(sender) = &*guard else {
            return Err(f)
        };
        let Some(error) = sender.try_send(f).err() else {
            return Ok(())
        };
        let Disconnected(f) = error else {
            return Err(f);
        };
        drop(guard);
        self.close();
        return Err(f);
    }

    pub fn is_closed(&self) -> bool {
        self.sender.read().unwrap_or_else(ignore_poison).is_none()
    }

    pub fn close(&self) {
        let mut sender = self.sender.write().unwrap_or_else(ignore_poison);
        *sender = None;
    }
}

fn ignore_poison<T>(e: PoisonError<T>) -> T {
    e.into_inner()
}
