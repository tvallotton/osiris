use super::unique_queue::{NoopHasher, UniqueQueue};
use super::waker::waker;
use crate::task::Task;
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
    // determines whether the block_on task has been woken.
    pub(crate) main_task: Cell<bool>,
    task_id: Cell<usize>,
}

impl Executor {
    pub fn new() -> Executor {
        Executor {
            tasks: RefCell::new(HashMap::with_capacity_and_hasher(4096, NoopHasher(0))),
            woken: RefCell::new(UniqueQueue::with_capacity(4096)),

            main_task: Cell::new(false),
            // we initialize it to one because 0 is reserved for the blocked_on task.
            task_id: Cell::new(1),
        }
    }

    pub fn block_on<F>(&self, mut future: F) -> F::Output
    where
        F: Future,
    {
        self.main_task.set(true);
        let waker = waker(0);
        let mut cx = Context::from_waker(&waker);
        loop {
            let future = unsafe { Pin::new_unchecked(&mut future) };
            if self.main_task.get() {
                match future.poll(&mut cx) {
                    Poll::Ready(ready) => return ready,
                    _ => {
                        self.poll_tasks();
                        self.park();
                    }
                }
            }
        }
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

    pub fn poll_tasks(&self) {
        loop {
            let mut woken = self.woken.borrow_mut();
            let task_id = woken.pop_front();

            if let Some(0) = task_id {
                self.main_task.set(true);
                return;
            } else if let Some(task_id) = task_id {
                // we drop woken so the task can call `.wake()`.
                drop(woken);

                let mut tasks = self.tasks.borrow_mut();
                if tasks.get(&task_id).clone().is_some()
                    && tasks.get(&(task_id + 1000000)).clone().is_some()
                {
                    panic!("");
                }

                if let Some(task) = tasks.remove(&task_id) {
                    // we drop tasks so the task can call `spawn`.
                    drop(tasks);

                    let waker = waker(task_id);
                    let ref mut cx = Context::from_waker(&waker);
                    match task.as_ref().poll(cx) {
                        Poll::Ready(()) => {}
                        Poll::Pending => {
                            // we insert it back into the queue.
                            self.tasks.borrow_mut().insert(task_id, task);
                        }
                    }
                }
            } else {
                break;
            }
        }
    }
    pub fn park(&self) {}
}
