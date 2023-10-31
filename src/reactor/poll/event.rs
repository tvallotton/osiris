use std::future::poll_fn;
use std::io::{self};
use std::task::Poll;

use crate::reactor::{self};

pub use libc::pollfd as Event;

pub struct Guard(u64);

impl Drop for Guard {
    fn drop(&mut self) {
        let reactor = reactor::current();
        let mut driver = reactor.driver();

        for i in 0..driver.wakers.len() {
            let (event_id, _) = &driver.wakers[i];
            if *event_id != self.0 {
                continue;
            }
            driver.wakers.swap_remove(i);
            driver.fds.swap_remove(i);
            break;
        }
    }
}

pub async fn wait(pollfd: Event) -> io::Result<()> {
    let mut submitted = false;
    let mut guard = None;
    poll_fn(|cx| {
        if submitted {
            return Poll::Ready(Ok(()));
        }
        submitted = true;
        let id = reactor::current().driver().push(pollfd, cx.waker().clone());
        guard = Some(Guard(id));
        Poll::Pending
    })
    .await
}

pub async fn submit<F, T>(event: libc::pollfd, mut f: F) -> io::Result<T>
where
    F: FnMut() -> io::Result<T>,
{
    loop {
        wait(event).await?;
        match f() {
            Err(err) => {
                let Some(libc::EAGAIN | libc::EINPROGRESS) = err.raw_os_error() else {
                    return Err(err);
                };
            }
            result => return result,
        }
    }
}

pub async fn submit_once<F>(kevent: libc::pollfd, f: F) -> io::Result<()>
where
    F: FnOnce() -> io::Result<i32>,
{
    match f() {
        Err(err) => {
            let Some(libc::EAGAIN | libc::EINPROGRESS) = err.raw_os_error() else {
                return Err(err);
            };
            wait(kevent).await
        }
        _ => Ok(()),
    }
}
