#![allow(unused_variables)]

use super::SharedTask;
use std::mem::forget;
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
    let wake = |data| {
        // Safety: same as above
        let task = unsafe { SharedTask::from_raw(data) };
    };
    let wake_by_ref = |data| {
        // Safety: same as above
        let task = unsafe { SharedTask::from_raw(data) };
        // let task = task.task().;
    };
    let drop = |data| todo!();
    RawWakerVTable::new(clone, wake, wake_by_ref, drop)
};
