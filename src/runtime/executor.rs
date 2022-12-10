use super::{Config, Runtime};
use crate::task::Task;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::future::{poll_fn, Future};
use std::mem::transmute;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::task::Context;

pub(crate) struct Executor {
    /// The run queue holds all tasks that are currently ready to do progress,
    /// either because they have been woken, or they were just spawned.
    pub(crate) queue: RefCell<VecDeque<Task>>,
    /// This bool states wheather the main task's JoinHandle has been woken.
    pub(crate) main_handle: Cell<bool>,
    /// A monotonically increasing counter for spawned tasks.
    /// It always corresponds to an available task id.
    pub(crate) task_id: Cell<u64>,
}

fn catch_unwind<T>(f: impl FnOnce() -> T) -> Result<T, Box<dyn Any + Send>> {
    std::panic::catch_unwind(AssertUnwindSafe(f))
}

impl Executor {
    /// Creates a new executor
    pub fn new(Config { init_capacity, .. }: Config) -> Executor {
        Executor {
            queue: RefCell::new(VecDeque::with_capacity(init_capacity)),
            main_handle: Cell::new(true),
            task_id: Cell::default(),
        }
    }

    pub fn task_id(&self) -> u64 {
        let task_id = self.task_id.get();
        self.task_id.set(task_id.overflowing_add(1).0);
        task_id
    }

    pub fn spawn<F>(&self, future: F, rt: Runtime) -> Task
    where
        F: Future + 'static,
    {
        let mut queue = self.queue.borrow_mut();
        let task_id = self.task_id();
        let task = Task::new(future, task_id, rt);
        queue.push_back(task.clone());
        task
    }

    /// Spawns a non-'static future onto the runtime.
    /// # Safety
    /// The caller must guarantee that the `future: Pin<&mut F>` must outlive the spawned
    /// task and its join handle. Otherwise, a use after free will occur.
    #[must_use]
    pub unsafe fn spawn_unchecked<F>(&self, future: Pin<&mut F>, rt: Runtime) -> Task
    where
        F: Future,
    {
        // Safety:
        // this trick will let us upgrade the lifetime
        // of F into a 'static lifetime. The caller must
        // ensure this invariant is met.
        let ptr: *mut () = unsafe { transmute(future) };

        let future = poll_fn(move |cx| {
            // Safety: explained in the transmute above.
            let future: Pin<&mut F> = unsafe { transmute(ptr) };
            future.poll(cx)
        });
        self.spawn(future, rt)
    }

    /// It polls at most `ticks` futures. It may poll less futures than
    /// the specified number of ticks. If a future finishes or panics it will be
    /// permanently removed from the task queue.
    #[inline]
    pub fn poll(&self, ticks: u32, task_id: &Cell<Option<u64>>) {
        for _ in 0..ticks {
            // we retrieve the queue of woken tasks
            let mut run_queue = self.queue.borrow_mut();

            let Some(task) = run_queue.pop_front() else {
                break;
            };

            task_id.set(Some(task.id()));

            // we drop the run queue so the task is able to
            // spawn other tasks.
            drop(run_queue);

            let waker = task.clone().waker();
            let cx = &mut Context::from_waker(&waker);

            if let Err(payload) = catch_unwind(|| task.poll(cx)) {
                task.panic(payload);
            };
        }
    }

    /// returns true if there is no more work to do
    pub fn is_idle(&self) -> bool {
        self.queue.borrow().len() == 0
    }
}
