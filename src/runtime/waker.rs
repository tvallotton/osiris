use crate::runtime::current_unwrap;
use std::task::{RawWaker, RawWakerVTable, Waker};

/// creates a waker for the main task.
pub(crate) fn main_waker() -> Waker {
    // Safety:
    // `data: *const ()` is Copy so no resources need to be managed.
    // RawWaker is thread safe so Wakers are thread safe.
    unsafe { Waker::from_raw(main_raw_waker()) }
}

fn main_raw_waker() -> RawWaker {
    RawWaker::new(std::ptr::null(), &MAIN_VTABLE)
}

const MAIN_VTABLE: RawWakerVTable = {
    let wake = |_| {
        current_unwrap("wake").executor.main_handle.set(true);
    };
    RawWakerVTable::new(|_| main_raw_waker(), wake, wake, |_| {})
};
