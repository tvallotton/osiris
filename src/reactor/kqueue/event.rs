use std::{future::Future, io};

use crate::reactor::{self, current};

struct Event<F> {
    submitted: bool,
    kevent: libc::kevent,
    f: F,
}

impl<F> Future for Event<F>
where
    F: Fn() -> io::Result<i32>,
{
    type Output = i32;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if !self.submitted {
            let reactor = reactor::current();
            let mut driver = reactor.driver();
            driver.push(kevent);
        }
    }
}

pub fn submit<F>(kevent: libc::kevent, f: F)
where
    F: Fn() -> io::Result<i32>,
{
}
