use std::any::Any;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::time::{Duration, Instant};

pub struct Work {
    pub id: u32,
    callback: Box<dyn FnOnce() -> Box<dyn Any + Send> + Send>,
}

pub struct WorkResult {
    pub id: u32,
    elapsed: Duration,
    res: Result<Box<dyn Any + Send>, Box<dyn Any + Send>>,
}

impl Work {
    pub fn new<F, T>(id: u32, callback: F) -> Self
    where
        F: FnOnce() -> T + Send,
        T: Send,
    {
        let callback = Box::new(|| Box::new(callback) as _);
        Work { id, callback }
    }

    pub fn execute(self) -> WorkResult {
        let time = Instant::now();
        let res = catch_unwind(AssertUnwindSafe(self.callback));
        WorkResult {
            id: self.id,
            elapsed: time.elapsed(),
            res,
        }
    }
}

impl WorkResult {
    pub fn unwrap<T>(self) -> Box<T> {
        match self.res {
            Ok(val) => val.downcast().unwrap(),
            Err(err) => resume_unwind(err),
        }
    }
}
