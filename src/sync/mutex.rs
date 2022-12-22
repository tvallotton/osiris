use crate::task::yield_now;
use std::cell::{Cell, RefCell, RefMut};
use std::collections::VecDeque;
use std::fmt::{Debug, Display};
use std::future::poll_fn;
use std::ops::{Deref, DerefMut};
use std::task::{Poll, Waker};

/// A mutual exclusion primitive useful for protecting shared data.
/// This mutex will block tasks waiting for the lock to become available.
/// The mutex can be created via a new constructor. Each mutex has a type
/// parameter which represents the data that it is protecting. The data can
/// only be accessed through the RAII guards returned from `lock` and `try_lock`,
/// which guarantees that the data is only ever accessed when the mutex is locked.
///
///  Unlike the `std::sync::Mutex` or the `tokio::sync::Mutex`, this `Mutex` does
/// not implement the `Send` and `Sync` traits. That is because this mutex's purpose
/// is to synchronize tasks, while those other mutexes are used to synchronize threads.
/// In general synchronizing tasks is cheaper than synchronizing threads. So generally,
/// when working with osiris tasks, this mutex should be preferred over std's or tokio's.
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
/// An RAII implementation of a “scoped lock” of a mutex.  When this structure
/// is dropped (falls out of scope),  the lock will be unlocked. The data
/// protected by the mutex can be accessed through this guard via its Deref and `DerefMut`
/// implementations. This structure is created by the `lock` and `try_lock` methods on `Mutex`.
pub struct Guard<'a, T> {
    value: RefMut<'a, T>,
    mutex: &'a Mutex<T>,
}

pub struct Error(());

impl<'a, T: Debug> Debug for Guard<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T: Debug> Debug for Mutex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.try_lock() {
            Ok(val) => f.debug_struct("Mutex").field("unlocked", &val).finish(),
            Err(_) => f.debug_struct("Mutex").field("locked", &"...").finish(),
        }
    }
}

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
    /// Creates a new mutex in an unlocked state ready for use.
    ///
    /// # Examples
    ///
    /// ```
    /// use osiris::sync::Mutex;
    ///
    /// let mutex = Mutex::new(0);
    /// ```
    pub fn new(value: T) -> Mutex<T> {
        Mutex {
            waiters: RefCell::default(),
            waiter_id: Cell::default(),
            value: RefCell::new(value),
        }
    }
    /// Acquires a mutex.
    ///
    /// This function will wait until the current task is able to acquire
    /// the mutex. Upon returning, the task is the only future with the lock
    /// held. An RAII guard is returned to allow scoped unlock of the lock. When
    /// the guard goes out of scope, the mutex will be unlocked.
    ///
    ///  Unlike the `std::sync::Mutex` or the `tokio::sync::Mutex`, this `Mutex` does
    /// not implement the `Send` and `Sync` traits. That is because this mutex's purpose
    /// is to synchronize tasks, while those other mutexes are used to synchronize threads.
    /// In general synchronizing tasks is cheaper than synchronizing threads. So generally,
    /// when working with osiris tasks, this mutex should be preferred over std's or tokio's.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::rc::Rc;
    /// use osiris::task;
    ///
    /// let mutex = Rc::new(Mutex::new(0));
    /// let c_mutex = Rc::clone(&mutex);
    /// # block_on(async move {
    /// spawn(async move {
    ///     *c_mutex.lock().unwrap() = 10;
    /// }).await;
    /// assert_eq!(*mutex.lock().await, 10);
    /// # });
    /// ```
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

    /// Attempts to acquire this lock.
    ///
    /// If the lock could not be acquired at this time, then [`Err`] is returned.
    /// Otherwise, an RAII guard is returned. The lock will be unlocked when the
    /// guard is dropped.
    ///
    /// # Errors
    ///
    /// If the mutex could not be acquired because it is already locked, then
    /// this call will return an error.
    ///
    /// # Examples
    ///
    /// ```
    /// # use osiris::block_on;
    /// use std::rc::Rc;
    /// use osiris::{sync::Mutex, spawn};
    ///
    /// let mutex = Rc::new(Mutex::new(0));
    /// let c_mutex = Rc::clone(&mutex);
    /// # block_on(async {
    /// spawn(async move {
    ///     let mut lock = c_mutex.try_lock();
    ///     if let Ok(ref mut mutex) = lock {
    ///         **mutex = 10;
    ///     } else {
    ///         println!("try_lock failed");
    ///     }
    /// }).await;
    /// assert_eq!(*mutex.lock().await, 10);
    /// # }).unwrap();
    /// ```
    pub fn try_lock(&self) -> Result<Guard<'_, T>, Error> {
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
