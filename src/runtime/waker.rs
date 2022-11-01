use crate::runtime::current_unwrap;
use std::task::{RawWaker, RawWakerVTable, Waker};

/// casts a usize to a pointer
/// in a way that doesn't make miri mad
fn int_to_ptr(n: usize) -> *const () {
    let p: *const u8 = core::ptr::null();
    p.wrapping_add(n) as *const ()
}

/// creates a waker for a task
pub(crate) fn waker(task_id: usize) -> Waker {
    // Safety:
    // `data: *const ()` is Copy so no resources need to be managed.
    // RawWaker is thread safe so Wakers are thread safe.
    unsafe { Waker::from_raw(raw_waker(int_to_ptr(task_id))) }
}

const fn raw_waker(data: *const ()) -> RawWaker {
    RawWaker::new(data, &VTABLE)
}

const VTABLE: RawWakerVTable = {
    let wake = |data| {
        current_unwrap("wake")
            .executor
            .woken
            .borrow_mut()
            .push_back(data as _);
    };

    RawWakerVTable::new(raw_waker, wake, wake, |_| {})
};

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
