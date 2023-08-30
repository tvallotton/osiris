#![allow(unused_variables)]

use super::SharedTask;
use std::mem::{forget, size_of};
use std::task::{RawWaker, RawWakerVTable, Waker};

pub(crate) fn waker(task: SharedTask) -> Waker {
    let raw = raw_waker(task.into_ptr());
    // Safety: The raw waker API is satisfied.
    unsafe { Waker::from_raw(raw) }
}

fn raw_waker(data: *const ()) -> RawWaker {
    RawWaker::new(data, &RAW_WAKER_VTABLE)
}

const RAW_WAKER_VTABLE: RawWakerVTable = {
    let clone = |data| {
        // Safety: its the same as the input type
        let task = unsafe { SharedTask::from_raw(data) };
        let new = task.clone();
        forget(task);
        raw_waker(new.into_ptr())
    };

    let wake = |data| unsafe {
        // Safety: its the same as the input type
        let task = SharedTask::from_raw(data);

        if task.thread_id() == std::thread::current().id() {
            let executor = task.meta().rt.executor;
            let mut queue = executor.queue.borrow_mut();
            queue.push_back(task);
            return;
        }
        let sender = task.meta().rt.executor.sender.clone();
        let waker = task.waker();
        let mut buf = vec![0; size_of::<Waker>()];
        buf.as_mut_ptr().cast::<Waker>().write(waker);
        // detach(sender.write(buf));
        todo!()
    };

    let wake_by_ref = |data| {
        // Safety: its the same as the input type
        let task = unsafe { SharedTask::from_raw(data) };
        let new = task.clone();
        new.waker().wake();
        forget(task);
    };
    let drop = |data| {
        // Safety: its the same as the input type
        unsafe { SharedTask::from_raw(data) };
    };
    RawWakerVTable::new(clone, wake, wake_by_ref, drop)
};
