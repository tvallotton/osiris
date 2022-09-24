use super::waker::waker;
use crate::task::Task;
use std::collections::hash_map::Entry;
use std::collections::VecDeque;
use std::pin::Pin;

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    future::Future,
    rc::Rc,
    task::{Context, Poll},
};
pub(crate) struct Executor {
    tasks: RefCell<HashMap<usize, Pin<Rc<dyn Task>>>>,
    pub(crate) woken: RefCell<VecDeque<usize>>,
    // determines whether the block_on task has been woken.
    block_on: Cell<bool>,
    task_id: Cell<usize>,
}

impl Executor {
    pub fn new() -> Executor {
        Executor {
            tasks: RefCell::default(),
            woken: RefCell::default(),

            block_on: Cell::new(false),
            // we initialize it to one because 0 is reserved for the blocked_on task.
            task_id: Cell::new(1),
        }
    }

    pub fn block_on<F>(&self, mut future: F) -> F::Output
    where
        F: Future,
    {
        self.block_on.set(true); 
        let waker = waker(0);
        let mut cx = Context::from_waker(&waker);
        loop {
            let future = unsafe { Pin::new_unchecked(&mut future) };
            println!("foo"); 
            if self.block_on.get() {
                match future.poll(&mut cx) {
                    Poll::Ready(ready) => return ready,
                    _ => continue,
                }
            }
            // self.poll_tasks();
            // self.park();
        }
    }

    pub fn spawn<F>(&self, future: F) -> Pin<Rc<dyn Task>>
    where
        F: Future + 'static,
    {
        let mut queue = self.tasks.borrow_mut();
        loop {
            let task_id = self.task_id.get();
            self.task_id.set(task_id.overflowing_add(2).0);
            match queue.entry(task_id) {
                Entry::Vacant(entry) => {
                    let future = <dyn Task>::new(task_id, future);
                    entry.insert(future.clone());
                    return future;
                }
                _ => continue,
            }
        }
    }
    pub fn poll_tasks(&self) {
        todo!()
    }
    pub fn park(&self) {
        todo!()
    }
}
