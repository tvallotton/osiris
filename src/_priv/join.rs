use std::future::Future;

use super::{cast, JoinWaker};
use std::ops::ControlFlow;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

/// Waits on multiple concurrent branches, returning when **all** branches
/// complete.
///
/// The `join!` macro must be used inside of async functions, closures, and
/// blocks.
///
/// The `join!` macro takes a list of async expressions and evaluates them
/// concurrently on the same task. Each async expression evaluates to a future
/// and the futures from each expression are multiplexed on the current task.
///
/// When working with async expressions returning `Result`, `join!` will wait
/// for **all** branches complete regardless if any complete with `Err`. Use
/// [`try_join!`] to return early when `Err` is encountered.
///
/// [`try_join!`]: crate::try_join
///
/// # Implementation notes
/// Unlike other implementations, this `join!` will not poll spuriously every
/// joined future. This is achieved by allocating a shared waker, and swapping the
/// waker vtable for each child future. This allows us to avoid spurious polling
/// without performing one allocation per future.
///
/// ### Differences with spawn
///
/// When it comes to performance, `join!` is more memory efficient that [`spawn`],
/// because it only incurs in a single allocation instead of one per future. However,
/// `join!` might make the future transform more complex, and reduce the branch efficiency of the
/// poll implementation.
///
/// In practice, it is useful to use `join!` when the `'static` bound on `spawn` cannot
/// be easily satisfied.
///
/// [`spawn`]: crate::spawn
///
/// # Examples
///
/// Basic join with two branches
///
/// ```
/// async fn do_stuff_async() {
///     // async work
/// }
///
/// async fn more_async_work() {
///     // more here
/// }
///
/// #[osiris::main]
/// async fn main() {
///     let (first, second) = osiris::join!(
///         do_stuff_async(),
///         more_async_work());
///
///     // do something with the values
/// }
/// ```
#[macro_export]
macro_rules! join {
    ($($input:expr),* $(,)?) => {{
        async {
            let waker = std::future::poll_fn(|cx| std::task::Poll::Ready(cx.waker().clone())).await;
            let waker = std::sync::Arc::new($crate::_priv::JoinWaker::new(waker));
            let out = $crate::_priv::Join::<($($crate::join!(@ignore $input),)*)>::new(($($input,)*), waker);
            out.await
        }
        .await
    }};
    (@ignore $tokens:expr) => {
        _
    };
}

pub struct Join<T> {
    cells: Option<T>,
    waker: Arc<JoinWaker<0>>,
}

macro_rules! implement_future_for_tuple {
    (
        types: [$($types:ident,)*],
        digits: [$($index:tt,)*],
        labels: [$($label:tt,)*]
    ) => {

        #[allow(nonstandard_style, unused_variables, irrefutable_let_patterns)]
        impl<$($types,)*> Join<($($types,)*)>
        where
        $($types: Future,)* {
            pub fn new(($($types,)*): ($($types,)*), waker: Arc<JoinWaker<0>>) -> Join<($(ControlFlow<$types::Output, $types>,)*)> {
                Join {
                    cells: Some(($(ControlFlow::Continue($types),)*)),
                    waker,
                }
            }
        }

        #[allow(nonstandard_style, unused_variables, irrefutable_let_patterns, unreachable_code)]
        impl<$($types,)*> Future for Join<(
            $(ControlFlow<$types::Output, $types>,)*
        )>
        where
            $($types: Future,)*
        {
            type Output = ($($types::Output,)*);
            fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
                let join = unsafe { self.get_unchecked_mut() };

                let cells = &mut join.cells.as_mut().unwrap();

                $(
                    $label: {

                        let cell = &mut cells.$index;


                        let ControlFlow::Continue(ref mut f) = cell else {
                            break $label;
                        };

                        let mask = 1 << $index;

                        let woken = join.waker.1.fetch_and(!mask, Ordering::Acquire);

                        if woken & mask == 0 {
                            break $label;
                        }

                        let fut = unsafe { Pin::new_unchecked(f) };

                        let waker = join.waker.clone();

                        let waker: Arc<JoinWaker<$index>> = cast(waker);

                        let waker: Waker = waker.into();

                        let cx = &mut Context::from_waker(&waker);

                        let Poll::Ready(ready) = fut.poll(cx) else {
                            break $label;
                        };

                        *cell = ControlFlow::Break(ready);

                    }
                )*

                if !matches!(cells, ($(ControlFlow::Break($types),)*)) {
                    return Poll::Pending;
                }

                let ($(ControlFlow::Break($types),)*) = join.cells.take().unwrap() else {
                    unreachable!()
                };

                Poll::Ready(($($types,)*))
            }
        }

        implement_future_for_tuple! {
            @recurse
            types:  [$($types,)*],
            digits: [$($index,)*],
            labels: [$($label,)*]
        }

    };

     (
        @recurse
        types:  [],
        digits: [],
        labels: []
    ) => {};
    (
        @recurse
        types:  [$_types:ident, $($types:ident,)* ],
        digits: [$_index:tt, $($index:tt,)*    ],
        labels: [$_label:tt, $($label:tt,)*    ]
    ) => {
        implement_future_for_tuple! {
            types:  [$($types,)*],
            digits: [$($index,)*],
            labels: [$($label,)*]
        }
    };
}

implement_future_for_tuple! {
    types: [
        A0,A1,A2,A3,A4,A5,A6,A7,A8,A9,A10,A11,A12,
        A13,A14,A15,A16,A17,A18,A19,A20,A21,A22,A23,
        A24,A25,A26,A27,A28,A29,A30,A31,
    ],
    digits: [
       31,30,29,28,27,26,25,24,23,22,21,20,19,18,17,16,15,14,13,12,11,10,9,8,7,6,5,4,3,2,1,0,
    ],
    labels: [
        'a0,'a1,'a2,'a3,'a4,'a5,'a6,'a7,'a8,'a9,'a10,'a11,'a12,'a13,'a14,
        'a15,'a16,'a17,'a18,'a19,'a20,'a21,'a22,'a23,'a24,'a25,'a26,'a27,
        'a28,'a29,'a30,'a31,

    ]
}
