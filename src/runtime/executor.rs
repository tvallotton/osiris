use super::config::Config;
use super::unique_queue::{NoopHasher, UniqueQueue};
use super::waker::waker;
use crate::task::{JoinHandle, Task};
use std::marker::PhantomData;
use std::mem::transmute;
use std::pin::Pin;

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    future::Future,
    rc::Rc,
    task::{Context, Poll},
};
pub(crate) struct Executor {
    tasks: RefCell<HashMap<usize, Pin<Rc<dyn Task>>, NoopHasher>>,
    pub(crate) woken: RefCell<UniqueQueue<usize>>,

    task_id: Cell<usize>,
}

impl Executor {
    pub fn new() -> Executor {
        Executor {
            tasks: RefCell::new(HashMap::with_capacity_and_hasher(4096, NoopHasher(0))),
            woken: RefCell::new(UniqueQueue::with_capacity(4096)),

            // we initialize it to one because 0 is reserved for the blocked_on task.
            task_id: Cell::new(1),
        }
    }

    /// 
    /// # Safety
    /// The runtime cannot outlive the future.
    ///
    #[inline(never)]
    pub unsafe fn block_on_spawn<F>(&self, future: Pin<&mut F>) -> JoinHandle<F::Output>
    where
        F: Future,
    {
        let ptr: *mut () = transmute(future);
        let future = std::future::poll_fn(move |cx| {
            let future: Pin<&mut F> = transmute(ptr);
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
        return future;
    }
    /// It polls at most `ticks` futures. It may poll less futures than
    /// the specified number of ticks. If a future finishes it will be
    /// permanently removed from the task queue.
    #[inline]
    pub fn poll(&self, ticks: u32) {
        for _ in 0..ticks {
            // we retrieve the queue of woken tasks
            let mut woken = self.woken.borrow_mut();

            let task_id = woken.pop_front();
            if let Some(task_id) = task_id {
                // we drop woken so the task can call `.wake()`.
                drop(woken);

                let mut tasks = self.tasks.borrow_mut();

                if let Some(task) = tasks.remove(&task_id) {
                    // we drop tasks so the task can call `spawn`.
                    drop(tasks);

                    let waker = waker(task_id);
                    let ref mut cx = Context::from_waker(&waker);
                    let poll = task.as_ref().poll(cx);

                    if let Poll::Pending = poll {
                        // we insert it back into the queue.
                        self.tasks.borrow_mut().insert(task_id, task);
                    }
                }
            }
        }
    }

    pub fn is_woken(&self) -> bool {
        self.woken.borrow().len() > 0
    }
}
