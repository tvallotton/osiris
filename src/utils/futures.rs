use std::any::Any;
use std::future::{poll_fn, Future};
use std::panic::{self, AssertUnwindSafe};
use std::pin::Pin;
use std::task::Poll;

pub async fn catch_unwind<F>(mut f: F) -> Result<F::Output, Box<dyn Any + Send + 'static>>
where
    F: Future,
{
    poll_fn(move |cx| {
        let closure = || unsafe { Pin::new_unchecked(&mut f).poll(cx) };
        match panic::catch_unwind(AssertUnwindSafe(closure)) {
            Err(err) => Poll::Ready(Err(err)),
            Ok(Poll::Pending) => Poll::Pending,
            Ok(Poll::Ready(value)) => Poll::Ready(Ok(value)),
        }
    })
    .await
}
