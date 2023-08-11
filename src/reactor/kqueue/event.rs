use std::future::poll_fn;
use std::io::{self};
use std::task::Poll;

use crate::reactor::{self};

pub(crate) fn id(event: libc::kevent) -> (usize, i16) {
    (event.ident, event.filter)
}

pub struct Guard(libc::kevent);
impl Drop for Guard {
    fn drop(&mut self) {
        let reactor = reactor::current();
        reactor.driver().wakers.remove(&id(self.0));
        // we don't delete the event
        // from the queue because some
        // other task may have also submitted
        // and event, and they would end up
        // waiting forever
    }
}

pub async fn wait(kevent: libc::kevent) -> io::Result<()> {
    let mut submitted = false;
    let mut guard = None;
    poll_fn(|cx| {
        if submitted {
            return Poll::Ready(Ok(()));
        }
        submitted = true;
        reactor::current()
            .driver()
            .push(kevent, cx.waker().clone())?;
        guard = Some(Guard(kevent));
        Poll::Pending
    })
    .await
}

pub async fn submit<F, T>(kevent: libc::kevent, mut f: F) -> io::Result<T>
where
    F: FnMut() -> io::Result<T>,
{
    loop {
        wait(kevent).await?;
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

pub async fn submit_once<F>(kevent: libc::kevent, f: F) -> io::Result<()>
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
