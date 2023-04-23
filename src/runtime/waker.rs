use crate::runtime::current_unwrap;
use std::ptr::null;
use std::task::{RawWaker, RawWakerVTable, Waker};

/// creates a waker for the main task.
pub(crate) fn main_waker() -> Waker {
    // Safety:
    // data: *const () is never accessed
    unsafe { Waker::from_raw(main_raw_waker()) }
}

fn main_raw_waker() -> RawWaker {
    RawWaker::new(null(), &MAIN_VTABLE)
}

const MAIN_VTABLE: RawWakerVTable = {
    let wake = |_| {
        current_unwrap("wake").executor.main_handle.set(true);
    };
    RawWakerVTable::new(|_| main_raw_waker(), wake, wake, |_| {})
};
