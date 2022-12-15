use crate::task::yield_now;
use std::cell::{Cell, RefCell, RefMut};
use std::collections::VecDeque;
use std::fmt::{Debug, Display};
use std::future::poll_fn;
use std::ops::{Deref, DerefMut};
use std::task::{Poll, Waker};

#[derive(Default)]
pub struct Mutex<T> {
    waiters: RefCell<VecDeque<(u64, Waker)>>,
    waiter_id: Cell<u64>,
    value: RefCell<T>,
}

struct Handle<'a, T> {
    mutex: &'a Mutex<T>,
    id: u64,
}

pub struct Guard<'a, T> {
    value: RefMut<'a, T>,
    mutex: &'a Mutex<T>,
}

pub struct Error(());

impl<'a, T> Deref for Guard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
impl<'a, T> DerefMut for Guard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "`try_lock()` failed because the mutex was locked.")
    }
}
impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TryLockError: \"{self}\"")
    }
}
// This drop implementation makes sure to wake any
// futures that are waiting to acquire the lock.
impl<'a, T> Drop for Guard<'a, T> {
    fn drop(&mut self) {
        let item = self.mutex.waiters.borrow_mut().pop_front();
        if let Some((_, waker)) = item {
            waker.wake();
        }
    }
}
/// This drop implementation makes sure that if the future gets
/// dropped, then it will remove its waker from the waiter queue.
/// If their waker was not found on the queue, then it must wake
/// another task, because that can only happen if it was the current
/// future's turn to acquire the lock.
impl<'a, T> Drop for Handle<'a, T> {
    fn drop(&mut self) {
        let mut waiters = self.mutex.waiters.borrow_mut();
        let start_len = waiters.len();
        waiters.retain(|&(id, _)| id != self.id);
        if start_len == waiters.len() {
            if let Some((_, waker)) = waiters.pop_front() {
                waker.wake();
            }
        }
    }
}

impl<T> Mutex<T> {
    // Creates a new lock in an unlocked state ready for use.
    fn new(value: T) -> Mutex<T> {
        Mutex {
            waiters: RefCell::default(),
            waiter_id: Cell::default(),
            value: RefCell::new(value),
        }
    }
    // A future that resolves on acquiring the lock and returns the MutexGuard.
    pub async fn lock(&self) -> Guard<'_, T> {
        let mut handle: Option<Handle<T>> = None;
        yield_now().await;
        poll_fn(move |cx| {
            if let Ok(val) = self.try_lock() {
                if let Some(handle) = handle.take() {
                    std::mem::forget(handle);
                }
                return Poll::Ready(val);
            }
            if handle.is_none() {
                handle = Some(self.push(cx.waker().clone()));
            };
            Poll::Pending
        })
        .await
    }

    fn try_lock(&self) -> Result<Guard<'_, T>, Error> {
        let Ok(value) = self.value.try_borrow_mut() else {
            return Err(Error(()));
        };
        Ok(Guard { value, mutex: self })
    }

    #[inline]
    fn push(&self, waker: Waker) -> Handle<T> {
        let id = self.id();
        self.waiters.borrow_mut().push_back((id, waker));
        Handle { mutex: self, id }
    }

    #[inline]
    fn id(&self) -> u64 {
        let id = self.waiter_id.get();
        self.waiter_id.set(id + 1);
        id
    }
}

#[cfg(not(miri))]
#[test]
fn mutex_stress_test() {
    use crate::task::yield_now;
    use crate::{block_on, spawn};
    use std::rc::Rc;
    use std::time::Instant;

    fn random() -> bool {
        thread_local! {static START : Instant =Instant::now() };
        START.with(|time| time.elapsed().as_nanos() % 61 < 61 / 2)
    }

    let mutex = Rc::new(Mutex::new(10));

    block_on(async {
        let mut handles = VecDeque::new();
        for _ in 0..10000 {
            let mutex = mutex.clone();
            if random() {
                handles.push_back(spawn(async move {
                    let mut number = mutex.lock().await;
                    yield_now().await;
                    yield_now().await;
                    yield_now().await;
                    *number += 1;
                }));
            } else {
                handles.pop_front();
            }
            yield_now().await;
        }
    })
    .unwrap();
    assert!(mutex.try_lock().is_ok());
}
