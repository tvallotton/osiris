use std::future::{poll_fn, Future};
use std::pin::Pin;
use std::task::{ready, Poll};
use std::time::Duration;

pub fn sleep(time: Duration) -> impl Future<Output = ()> + Unpin {
    use crate::shared_driver::submit;
    use io_uring::opcode::Timeout;
    use io_uring::types::Timespec;
    let timespec = Timespec::new()
        .sec(time.as_secs())
        .nsec(time.subsec_nanos());
    let timespec = Box::new(timespec);
    let entry = Timeout::new(&*timespec as *const Timespec)
        .count(u32::MAX)
        .build();
    // Safety:
    let mut event = unsafe { submit(entry, timespec) };
    poll_fn(move |cx| {
        ready!(Pin::new(&mut event).poll(cx)).0.unwrap();
        Poll::Ready(())
    })
}
