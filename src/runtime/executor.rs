use super::unique_queue::UniqueQueue;
use super::waker::waker;
use crate::hasher::NoopHasher;
use crate::task::{JoinHandle, Task};

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::{poll_fn, Future};
use std::mem::transmute;
use std::pin::Pin;
use std::rc::Rc;
use std::task::Context;

pub(crate) struct Executor {
    /// This collection stores all the tasks spawned onto the runtime.
    pub(crate) tasks: RefCell<Tasks>,
    /// This is queue stores all woken tasks in the order they were
    /// woken.
    pub(crate) woken: RefCell<UniqueQueue<usize>>,
    /// This queue stores the ids for all aborted tasks.
    pub(crate) aborted: RefCell<UniqueQueue<usize>>,
    /// This bool states wheather the main task's JoinHandle has been woken.
    /// This will be true when the main task has finished.
    pub(crate) main_handle: Cell<bool>,
    /// A monotonically increasing counter for spawned tasks.
    /// It always corresponds to an available task id.
    pub(crate) task_id: Cell<usize>,
}

type Tasks = HashMap<usize, Pin<Rc<dyn Task>>, NoopHasher>;

impl Executor {
    /// Creates a new executor
    pub fn new() -> Executor {
        Executor {
            tasks: RefCell::new(HashMap::with_capacity_and_hasher(4096, NoopHasher::new())),
            woken: RefCell::new(UniqueQueue::with_capacity(4096)),
            aborted: RefCell::new(UniqueQueue::with_capacity(4096)),
            main_handle: Cell::new(true),
            // we initialize it to one because 0 is reserved for the blocked_on task.
            task_id: Cell::new(1),
        }
    }

    /// # Safety
    /// The caller must guarantee that the `future: Pin<&mut F>` must outlive the spawned
    /// task. Otherwise, a use after free will occur.    
    pub unsafe fn block_on_spawn<F>(&self, future: Pin<&mut F>) -> JoinHandle<F::Output>
    where
        F: Future,
    {
        // SAFETY:
        // this trick will let us upgrade the lifetime
        // of F into a 'static lifetime. The caller must
        // ensure this invariant is met.
        let ptr: *mut () = unsafe { transmute(future) };

        let future = poll_fn(move |cx| {
            // SAFETY: explained in the transmute above.
            let future: Pin<&mut F> = unsafe { transmute(ptr) };
            future.poll(cx)
        });

        let task = <dyn Task>::new(0, future);
        self.tasks.borrow_mut().insert(0, task.clone());

        JoinHandle::new(task)
    }

    pub fn spawn<F>(&self, future: F) -> Pin<Rc<dyn Task>>
    where
        F: Future + 'static,
    {
        let mut queue = self.tasks.borrow_mut();

        let task_id = self.task_id.get();
        self.task_id.set(task_id.overflowing_add(2).0);
        let future = <dyn Task>::new(task_id, future);
        queue.insert(task_id, future.clone());
        waker(task_id).wake();
        future
    }
    /// It polls at most `ticks` futures. It may poll less futures than
    /// the specified number of ticks. If a future finishes or panics it will be
    /// permanently removed from the task queue.
    #[inline]
    pub fn poll(&self, ticks: u32, task_id: &Cell<Option<usize>>) {
        for _ in 0..ticks {
            // we retrieve the queue of woken tasks
            let mut woken = self.woken.borrow_mut();

            task_id.set(woken.pop_front());

            if let Some(task_id) = task_id.get() {
                // we drop woken so the task can call `.wake()`.
                drop(woken);

                let mut tasks = self.tasks.borrow_mut();

                if let Some(task) = tasks.remove(&task_id) {
                    // we drop tasks so the task can call `spawn`.
                    drop(tasks);

                    let waker = waker(task_id);
                    let cx = &mut Context::from_waker(&waker);
                    let poll = task.as_ref().poll(cx);

                    if poll.is_pending() {
                        // we insert it back into the queue.
                        self.tasks.borrow_mut().insert(task_id, task);
                    }
                }
            }
        }
    }
    /// Removes aborted tasks from the executor.
    pub fn remove_aborted(&self) {
        loop {
            let mut aborted = self.aborted.borrow_mut();
            if let Some(task_id) = aborted.pop_front() {
                let mut tasks = self.tasks.borrow_mut();
                let task = tasks.remove(&task_id);
                // we first drop the task queue,
                // in case the task destructors wants to spawn other futures
                drop(tasks);
                // we now drop the aborted queue in case the task destructor
                // wants to abort another task.
                drop(aborted);
                drop(task);
            }
        }
    }

    pub fn is_woken(&self) -> bool {
        self.woken.borrow().len() > 0
    }
}
