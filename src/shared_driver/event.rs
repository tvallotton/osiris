#![allow(warnings)]
use super::{driver, SharedDriver};
use crate::detach;
use crate::runtime::current;
#[cfg(target_os = "linux")]
use io_uring::cqueue;
use io_uring::opcode::AsyncCancel;
#[cfg(target_os = "linux")]
use io_uring::squeue;
#[cfg(target_os = "linux")]
use io_uring::squeue::Entry;
use std::future::{poll_fn, Future};
use std::io;
use std::mem::forget;
use std::ops::ControlFlow;
use std::ops::ControlFlow::*;
use std::pin::Pin;
use std::process::Output;
use std::task::{Context, Poll};
use std::thread::panicking;

/// A future for IO events
///
/// Dropping this future will automatically spawn an abort task
/// if the io wasn't completed
pub struct Event<T: 'static> {
    entry: Option<squeue::Entry>,
    driver: SharedDriver,
    id: u64,
    data: Option<T>,
    requires_cancel: bool,
}

type Sub<T> = (Result<cqueue::Entry, io::Error>, T);

impl<T> Unpin for Event<T> {}

impl<T> Future for Event<T> {
    type Output = Sub<T>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(entry) = self.entry.take() {
            // Safety: invariants upheld at construction
            match unsafe { self.driver.push(entry) } {
                Ok(id) => self.id = id,
                Err(err) => return Poll::Ready((Err(err), self.data.take().unwrap())),
            };
            self.requires_cancel = true;
        }

        let Poll::Ready(entry) = self.driver.poll(self.id, cx) else {
            return Poll::Pending;
        };
        self.requires_cancel = false;
        let data = self.data.take().unwrap();
        Poll::Ready((Ok(entry), data))
    }
}

impl<T: 'static> Drop for Event<T> {
    fn drop(&mut self) {
        if self.requires_cancel {
            let entry = io_uring::opcode::AsyncCancel::new(self.id).build();
            let data = self.data.take().unwrap();
            let Some(rt) = current() else {
                forget(data);
                let msg = "memory leak detected. failed to spawn cleanup task while cancelling a future."; 
                if !panicking() {
                    panic!("{msg}"); 
                } else {
                    eprintln!("error: {msg}"); 
                }
                return;
            };
            let cancel = unsafe { submit(entry, data) };
            rt.executor.spawn(cancel, rt.clone(), true);
        }
    }
}

#[cfg(target_os = "linux")]
pub(crate) unsafe fn submit<T: 'static>(
    entry: Entry,
    data: T,
) -> impl Future<Output = Sub<T>> + Unpin {
    Event {
        entry: Some(entry),
        driver: super::current(),
        data: Some(data),
        id: 0,
        requires_cancel: false,
    }
}
