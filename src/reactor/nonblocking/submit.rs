use crate::reactor::{self, Event};
use std::future::poll_fn;
use std::io::{self};
use std::task::Poll;

pub struct Guard(u64);

impl Drop for Guard {
    fn drop(&mut self) {
        let reactor = reactor::current();
        let mut driver = reactor.driver();
        driver.remove_waker(self.0);
    }
}

pub async fn wait(event: Event) -> io::Result<()> {
    let mut submitted = false;
    let mut guard = None;
    poll_fn(|cx| {
        if submitted {
            return Poll::Ready(Ok(()));
        }
        submitted = true;
        let id = reactor::current().driver().push(event, cx.waker().clone());
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

/// Note: this function is used for connect mostly.
/// In connect the system call needs to be performed before the wait, however,
/// after the wait,
pub async fn submit_once<F>(event: libc::pollfd, f: F) -> io::Result<()>
where
    F: FnOnce() -> io::Result<i32>,
{
    match f() {
        Err(err) => {
            let Some(libc::EAGAIN | libc::EINPROGRESS) = err.raw_os_error() else {
                return Err(err);
            };
            wait(event).await
        }
        _ => Ok(()),
    }
}
