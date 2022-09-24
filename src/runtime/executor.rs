use super::waker::waker;
use crate::pin;
use std::collections::VecDeque;
use std::pin::Pin;
use std::task::ready;
use std::{
    cell::{Cell, RefCell},
    collections::{
        hash_map::{Entry, VacantEntry},
        HashMap,
    },
    future::Future,
    rc::Rc,
    task::{Context, Poll, Waker},
};
pub(crate) struct Executor {
    tasks: RefCell<HashMap<usize, Rc<dyn Task>>>,
    pub(crate) woken: RefCell<VecDeque<usize>>,
    // determines whether the block_on task has been woken.
    block_on: Cell<bool>,
    task_id: Cell<usize>,
}

pub(crate) trait Task {}

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
        let waker = waker(0);
        let mut cx = Context::from_waker(&waker);
        loop {
            let future = unsafe { Pin::new_unchecked(&mut future) };
            if self.block_on.get() {
                match future.poll(&mut cx) {
                    Poll::Ready(ready) => return ready,
                    _ => continue,
                }
            }
            self.poll_tasks();
            self.park();
        }
    }

    pub fn poll_tasks(&self) {
        todo!()
    }
    pub fn park(&self) {
        todo!()
    }
}
