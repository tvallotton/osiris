use std::task::{RawWaker, RawWakerVTable, Waker};

pub(crate) fn waker(task_id: usize) -> Waker {
    unsafe { Waker::from_raw(raw_waker(task_id as _)) }
}

const fn raw_waker(data: *const ()) -> RawWaker {
    RawWaker::new(data, &VTABLE)
}

const VTABLE: RawWakerVTable = {
    let clone = raw_waker;
    let drop = |_| {};
    let wake_by_ref = |_| todo!();
    let wake = wake_by_ref;

    RawWakerVTable::new(clone, wake, wake_by_ref, drop)
};
