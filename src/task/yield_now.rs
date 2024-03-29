use std::future::{poll_fn, Future};
use std::task::Poll;

/// Yields execution back to the runtime.
///
/// A task yields by awaiting on `yield_now()`, and may resume when that future
/// completes (with no output.) The current task will be re-added as a pending
/// task at the _back_ of the pending queue. Any other pending tasks will be
/// scheduled. No other waking is required for the task to continue.
///
///
///
/// ## Non-guarantees
///
/// This function may not yield all the way up to the executor if there are any
/// special combinators above it in the call stack. For example, if a
/// `select!` has another branch complete during the same poll as the
/// `yield_now()`, then the yield is not propagated all the way up to the
/// runtime.
///
/// It is generally not guaranteed that the runtime behaves like you expect it
/// to when deciding which task to schedule next after a call to `yield_now()`.
/// In particular, the runtime may choose to poll the task that just ran
/// `yield_now()` again immediately without polling any other tasks first. For
/// example, the runtime will not drive the IO driver between every poll of a
/// task, and this could result in the runtime polling the current task again
/// immediately even if there is another task that could make progress if that
/// other task is waiting for a notification from the IO driver.
///
/// In general, changes to the order in which the runtime polls tasks is not
/// considered a breaking change, and your program should be correct no matter
/// which order the runtime polls your tasks in.
///
#[cfg_attr(docsrs, doc(cfg(feature = "rt")))]
pub fn yield_now() -> impl Future<Output = ()> + Unpin {
    let mut ready = false;
    poll_fn(move |cx| {
        if ready {
            Poll::Ready(())
        } else {
            ready = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    })
}
