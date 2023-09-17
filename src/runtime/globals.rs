use super::{thread_pool::ThreadPool, Runtime};
use std::{
    cell::{Cell, RefCell},
    sync::OnceLock,
};

thread_local! {
    /// This is the runtime thread local. It determines in which runtime context we are currently in.
    pub(crate) static RUNTIME: RefCell<Option<Runtime>>= RefCell::new(None);
}

thread_local! {
    /// This is the task thread local. It determines which task is currently being executed.
    pub(crate) static TASK_ID: Cell<Option<u64>> = Cell::new(None);
}

pub(crate) static THREAD_POOL: OnceLock<ThreadPool> = OnceLock::new();
