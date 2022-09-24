use crate::runtime::current_unwrap;
use std::task::{RawWaker, RawWakerVTable, Waker};



pub(crate) fn waker(task_id: usize) -> Waker {
    unsafe { Waker::from_raw(raw_waker(task_id as _)) }
}

const fn raw_waker(data: *const ()) -> RawWaker {
    RawWaker::new(data, &VTABLE)
}

const VTABLE: RawWakerVTable = {
    let wake = |data| {
        current_unwrap("wake")
            .0
            .executor
            .woken
            .borrow_mut()
            .push_back(data as _)
    };
    RawWakerVTable::new(raw_waker, wake, wake, |_| {})
};
