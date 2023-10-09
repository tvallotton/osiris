#![allow(unused_variables)]

use crate::runtime::current;

use super::SharedTask;
use std::mem::{forget, size_of, transmute};
use std::task::{RawWaker, RawWakerVTable, Waker};

pub(crate) fn waker(task: SharedTask) -> Waker {
    let raw = raw_waker(task.into_ptr());
    // Safety: The raw waker API is satisfied.
    unsafe { Waker::from_raw(raw) }
}

fn raw_waker(data: *const ()) -> RawWaker {
    RawWaker::new(data, &RAW_WAKER_VTABLE)
}

unsafe fn wake_by_ref(data: *const ()) {
    // Safety: its the same as the input type
    let task = unsafe { SharedTask::from_raw(data) };
    let new = task.clone();
    new.waker().wake();
    forget(task);
}

unsafe fn wake(data: *const ()) {
    // Safety: its the same as the input type
    let task = SharedTask::from_raw(data);
    let is_same_thread = task.thread_id() == std::thread::current().id();
    if is_same_thread {
        return wake_local(task);
    }
    if let Some(rt) = current() {
        rt._spawn(wake_multithread(task), true).detach();
    } else {
        wake_multithread_blocking(task);
    }
}

unsafe fn wake_local(task: SharedTask) {
    let executor = task.meta().rt.executor;
    let mut queue = executor.queue.borrow_mut();
    queue.push_back(task);
}

async unsafe fn wake_multithread(task: SharedTask) {
    let sender = task.meta().rt.executor.sender.clone();
    let waker = task.waker();
    let mut buf = [0; size_of::<Waker>()];
    buf.as_mut_ptr().cast::<Waker>().write(waker);
    let result = sender.write_nonblock(&buf).await;
    if let Err(err) = result {
        let _: Waker = transmute(buf);
        panic!("failed to wake task: {err}");
    }
}

unsafe fn wake_multithread_blocking(task: SharedTask) {
    let sender = task.meta().rt.executor.sender.clone();
    let waker = task.waker();
    let mut buf = [0; size_of::<Waker>()];
    buf.as_mut_ptr().cast::<Waker>().write(waker);
    if let Err(err) = sender.write_block(&buf) {
        let _: Waker = transmute(buf);
        panic!("failed to wake task: {err}");
    }
}

const RAW_WAKER_VTABLE: RawWakerVTable = {
    let clone = |data| {
        // Safety: its the same as the input type
        let task = unsafe { SharedTask::from_raw(data) };
        let new = task.clone();
        forget(task);
        raw_waker(new.into_ptr())
    };

    let drop = |data| {
        // Safety: its the same as the input type
        unsafe { SharedTask::from_raw(data) };
    };
    RawWakerVTable::new(clone, wake, wake_by_ref, drop)
};
