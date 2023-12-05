use crate::buf::IoBuf;
use crate::net::pipe;
use crate::runtime::current_unwrap;
use std::mem::size_of;
use std::panic::catch_unwind;
use std::ptr::null;
use std::rc::Rc;
use std::task::{RawWaker, RawWakerVTable, Waker};

/// creates a waker for the main task.
#[inline]
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

/// This function will receive wakers from other threads using
/// the async pipe, and it will call wake on those wakers
pub(crate) async fn forward_multithreaded_wakeups(receiver: Rc<pipe::Receiver>) {
    const WAKER_SIZE: usize = size_of::<Waker>();
    let mut data = vec![0u8; WAKER_SIZE];
    loop {
        let mut read = 0;
        while read < WAKER_SIZE {
            // we attempt to read
            let (res, buf) = receiver.read(data.slice(read..(WAKER_SIZE - read))).await;
            data = buf.into_inner();

            let Ok(additional) = res else {
                return;
            };

            read += additional;
        }

        let data: *mut Waker = data.as_mut_ptr().cast();
        let waker = unsafe { std::ptr::read(data) };

        catch_unwind(|| waker.wake()).ok();
    }
}
