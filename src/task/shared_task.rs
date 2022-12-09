#![allow(
    clippy::cast_possible_wrap,
    clippy::ptr_offset_with_cast,
    clippy::cast_ptr_alignment,
    clippy::enum_glob_use
)]
use crate::runtime::Runtime;

use super::raw_task::RawTask;
use super::task_repr::TaskRepr;
use std::alloc::{dealloc, Layout};
use std::future::Future;
use std::mem::forget;
use std::pin::Pin;
use std::ptr::drop_in_place;
use std::sync::atomic::Ordering::*;
use std::sync::atomic::{self, AtomicUsize};
use std::thread::{current, ThreadId};

/// This is a manually reference counted task. It is intended
/// to work as an Arc<dyn Task>, except it is a thin pointer, so
/// it fits in the Waker's `data: *const ()` field in a single
/// allocation.
///
/// Even though shared tasks are Send, they do not support being
/// sent across threads, and attempting to do so will cause runtime panics
/// and the memory to be leaked. This can occur if a waker is sent to another thread and
/// woken or dropped from that thread.
pub(crate) struct SharedTask {
    /// the memory allocation
    data: *const Inner,
}

#[repr(C)]
struct Inner {
    /// the id for the thread where the Task was constructed
    thread_id: ThreadId,
    /// the reference count
    count: AtomicUsize,
    task: *const dyn RawTask,
}

/// Safety: The reference count is synchronized, and
/// the task is innaccessible in other threads
unsafe impl Send for SharedTask {}

/// Takes a task and it returns the layout required for the allocation.
/// The layout returned can be represented roughly as:
/// ```
/// struct Inner {
///     thread_id: ThreadId,
///     coint: AtomicUsize,
///     task: *const dyn RawTask,
///     task_alloc: dyn RawTask,
/// }
/// ```
/// It works by extending the layout for `Inner` with the layout required for `T`.
fn alloc_layout<T: ?Sized>(task: &T) -> (Layout, isize) {
    const LAYOUT: Layout = Layout::new::<Inner>();
    let task_layout = Layout::for_value(task);
    let (layout, offset) = LAYOUT.extend(task_layout).unwrap();
    (layout, offset as _)
}

impl SharedTask {
    /// Creates a new shared task.
    pub fn new<F: Future + 'static>(f: F, rt: Runtime) -> Self {
        let task = TaskRepr::new(f, rt);
        SharedTask::from_raw_task(task)
    }
    pub fn into_ptr(self) -> *const () {
        let ptr = self.data.cast();
        forget(self);
        ptr
    }

    /// Takes a raw pointer and converts it into an owned [`SharedTask`]
    pub unsafe fn from_raw(ptr: *const ()) -> SharedTask {
        SharedTask { data: ptr.cast() }
    }
    /// Creates a new shared task from a raw task.
    fn from_raw_task<T: RawTask + 'static>(value: T) -> Self {
        let (alloc_layout, offset) = alloc_layout(&value);

        // Safety: the allocation size can't be zero because ArcInner isn't a ZST
        let data = unsafe { std::alloc::alloc(alloc_layout) };

        // Safety: we are writting to the offset we were given by the layout
        unsafe {
            data.offset(offset).cast::<T>().write(value);
        };
        // drop(value);
        // Safety: we own the pointer and the layout is correct
        unsafe {
            data.cast::<Inner>().write(Inner {
                thread_id: current().id(),
                count: AtomicUsize::new(1),
                task: data.offset(offset).cast::<T>() as *const dyn RawTask,
            });
        }
        SharedTask { data: data.cast() }
    }
}
impl SharedTask {
    #[inline]
    fn inner(&self) -> &Inner {
        // Safety: This is ok because while this arc is alive we're guaranteed
        // that the inner pointer is valid.
        unsafe { &*self.data }
    }

    pub fn task(&self) -> Pin<&dyn RawTask> {
        let task = self.inner();

        assert_eq!(task.thread_id, current().id(),
        "osiris wakers and join handles should not be shared between threads. If you do, make sure to use them and drop them in the thread they where created."
        );
        // Safety: SharedTasks are always structurally pinned
        unsafe { Pin::new_unchecked(&*self.inner().task) }
    }
}
impl Drop for SharedTask {
    fn drop(&mut self) {
        let count = self.inner().count.fetch_sub(1, Release);
        if count != 1 {
            return;
        }
        // we make sure the task is being dropped from the correct thread.
        assert_eq!(self.inner().thread_id, current().id(), "A panic occured because a waker was dropped from another thread. Make sure all wakers are dropped in the same thread they were spawned in.");
        atomic::fence(Acquire);

        let task = &*self.task();

        let (layout, _) = alloc_layout(task);
        // Safety: we are the last reference, so it is ok to drop.
        unsafe { drop_in_place(self.inner().task as *mut dyn RawTask) };
        // Safety: we own this allocation.
        unsafe { dealloc(self.data as _, layout) }
    }
}

impl Clone for SharedTask {
    fn clone(&self) -> Self {
        self.inner().count.fetch_add(1, Relaxed);
        SharedTask { data: self.data }
    }
}
