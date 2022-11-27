use super::unique_queue::UniqueQueue;
use super::waker::waker;
use super::{Config, Runtime};
use crate::hasher::NoopHasher;
use crate::task::Task;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

use std::task::{Context, Poll};

pub(crate) struct Executor {
    /// This collection stores all the tasks spawned onto the runtime.
    pub(crate) tasks: RefCell<Tasks>,
    /// This queue stores all woken tasks in the order they were
    /// woken.
    pub(crate) woken: RefCell<UniqueQueue>,
    /// This bool states wheather the main task's JoinHandle has been woken.
    pub(crate) main_handle: Cell<bool>,
    /// A monotonically increasing counter for spawned tasks.
    /// It always corresponds to an available task id.
    pub(crate) task_id: Cell<usize>,
}

type Tasks = HashMap<usize, Task, NoopHasher>;

impl Executor {
    /// Creates a new executor
    pub fn new(Config { init_capacity, .. }: Config) -> Executor {
        Executor {
            tasks: RefCell::new(HashMap::with_capacity_and_hasher(
                init_capacity,
                NoopHasher::new(),
            )),
            woken: RefCell::new(UniqueQueue::with_capacity(init_capacity)),
            main_handle: Cell::new(true),
            task_id: Cell::default(),
        }
    }

    pub fn task_id(&self) -> usize {
        let task_id = self.task_id.get();
        self.task_id.set(task_id.overflowing_add(1).0);
        task_id
    }

    pub fn spawn<F>(&self, future: F, rt: Runtime) -> Task
    where
        F: Future + 'static,
    {
        let mut queue = self.tasks.borrow_mut();

        let task_id = self.task_id();
        let future = Task::new(task_id, future, rt);
        queue.insert(task_id, future.clone());
        waker(task_id).wake();
        future
    }
    /// It polls at most `ticks` futures. It may poll less futures than
    /// the specified number of ticks. If a future finishes or panics it will be
    /// permanently removed from the task queue.
    #[inline]
    pub fn poll(&self, ticks: u32, task_id: &Cell<Option<usize>>, main_id: usize) {
        for _ in 0..ticks {
            // we retrieve the queue of woken tasks
            let mut woken = self.woken.borrow_mut();

            task_id.set(woken.pop_front());

            let Some(task_id) = task_id.get() else {
                continue;
            };

            // we drop woken so the task can call `.wake()`.
            drop(woken);

            // we remove the task from the task map
            let Some(task) = self.tasks.borrow_mut().remove(&task_id) else {
                continue;
            };

            let waker = waker(task_id);
            let cx = &mut Context::from_waker(&waker);
            let poll = catch_unwind(AssertUnwindSafe(|| task.poll(cx)));

            match poll {
                Ok(Poll::Pending) => {
                    // we insert it back into the queue.
                    self.tasks.borrow_mut().insert(task_id, task);
                }
                Ok(Poll::Ready(())) => {
                    continue;
                }
                Err(error) if task_id == main_id => {
                    resume_unwind(error);
                }
                Err(error) => {
                    task.panicked(error);
                }
            }
        }
    }
    /// returns true if there is no more work to do
    pub fn is_idle(&self) -> bool {
        self.woken.borrow().len() == 0
    }
}
