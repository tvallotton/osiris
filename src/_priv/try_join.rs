use std::future::Future;
use std::marker::PhantomData;

use std::ops::ControlFlow;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use super::{cast, JoinWaker};

/// Waits on multiple concurrent branches, returning when **all** branches
/// complete with `Ok(_)` or on the first `Err(_)`.
///
/// The `try_join!` macro must be used inside of async functions, closures, and
/// blocks.
///
/// Similar to [`join!`], the `try_join!` macro takes a list of async
/// expressions and evaluates them concurrently on the same task. Each async
/// expression evaluates to a future and the futures from each expression are
/// multiplexed on the current task. The `try_join!` macro returns when **all**
/// branches return with `Ok` or when the **first** branch returns with `Err`.
///
///
/// # Implementation notes
/// This `try_join!` macro implementation has two advantages over other alternative
/// implementations:
/// 1. It does not poll spuriously (i.e. it doesn't poll branches that weren't woken).
/// 2. It doesn't require one allocation per future.
///
/// It however has one drawback when compared to Tokio's `try_join!` macro implemtation, and that
/// is that it incurs in a single memory allocation. This implementation effectively trades off
/// the spurious polling in exchange for a memory allocation.
///
/// [`spawn`]: crate::spawn
/// [`join!`]: crate::join!
///
/// # Examples
///
/// Basic try_join with two branches.
///
/// ```
/// async fn do_stuff_async() -> Result<i32, &'static str> {
///     // async work
///     
/// # Ok(1)
/// }
///
/// async fn more_async_work() -> Result<i32, &'static str> {
///     // more here
/// # Ok(2)
/// }
///
/// #[osiris::main]
/// async fn main() -> Result<(), &'static str> {
///     let (first, second) = osiris::try_join!(
///         do_stuff_async(),
///         more_async_work()
///     )?;
///     
///     assert_eq!(first, 1);
///     assert_eq!(second, 2);
///    
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! try_join {
    ($($input:expr),* $(,)?) => {{
        async {
            let waker = std::future::poll_fn(|cx| std::task::Poll::Ready(cx.waker().clone())).await;
            let waker = std::sync::Arc::new($crate::_priv::JoinWaker::<0>::new(waker));
            let out = $crate::_priv::TryJoin::<($($crate::join!(@ignore $input),)*), _>::new(($($input,)*), waker);
            out.await
        }
        .await
    }};
    (@ignore $tokens:expr) => {
        _
    };
}

pub struct TryJoin<T, E> {
    cells: Option<T>,
    waker: Arc<JoinWaker<0>>,
    _ph: PhantomData<E>,
}

macro_rules! implement_future_for_tuple {
    (
        future_types: [$($ftypes:ident,)*],
        output_types: [$($otypes:ident,)*],

        digits: [$($index:tt,)*],
        labels: [$($label:tt,)*]
    ) => {

        #[allow(nonstandard_style, unused_variables, irrefutable_let_patterns)]
        impl<E, $($ftypes,)* $($otypes,)*> TryJoin<($($ftypes,)*), E>
        where
        $($
            ftypes: Future<Output=Result<$otypes, E>>,
        )* {
            pub fn new(($($ftypes,)*): ($($ftypes,)*), waker: Arc<JoinWaker<0>>) -> TryJoin<($(ControlFlow<$otypes, $ftypes>,)*), E> {
                TryJoin {
                    cells: Some(($(ControlFlow::Continue($ftypes),)*)),
                    waker,
                    _ph: PhantomData
                }
            }
        }

        #[allow(nonstandard_style, unused_variables, irrefutable_let_patterns, unreachable_code)]
        impl<E, $($ftypes,)* $($otypes,)*> Future for TryJoin<(
            $(ControlFlow<$otypes, $ftypes>,)*
        ), E>
        where
            $($ftypes: Future<Output=Result<$otypes, E>>,)*
        {
            type Output = Result<($($otypes,)*), E>;

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

                        match ready {
                            Ok(val) => *cell = ControlFlow::Break(val),
                            Err(err) => return Poll::Ready(Err(err))
                        }
                    }
                )*

                if !matches!(cells, ($(ControlFlow::Break($otypes),)*)) {
                    return Poll::Pending;
                }

                let ($(ControlFlow::Break($otypes),)*) = join.cells.take().unwrap() else {
                    unreachable!()
                };

                Poll::Ready(Ok(($($otypes,)*)))
            }
        }

        implement_future_for_tuple! {
            @recurse
            future_types:  [$($ftypes,)*],
            output_types:  [$($otypes,)*],
            digits: [$($index,)*],
            labels: [$($label,)*]
        }

    };

     (
        @recurse
        future_types:  [],
        output_types: [],
        digits: [],
        labels: []
    ) => {};
    (
        @recurse
        future_types:  [$_ftypes:ident, $($ftypes:ident,)* ],
        output_types:  [$_otypes:ident, $($otypes:ident,)* ],
        digits: [$_index:tt, $($index:tt,)*],
        labels: [$_label:tt, $($label:tt,)*]
    ) => {
        implement_future_for_tuple! {
            future_types:  [$($ftypes,)*],
            output_types:  [$($otypes,)*],
            digits: [$($index,)*],
            labels: [$($label,)*]
        }
    };
}

implement_future_for_tuple! {
    future_types: [
        A0,A1,A2,A3,A4,A5,A6,A7,A8,A9,A10,A11,A12,
        A13,A14,A15,A16,A17,A18,A19,A20,A21,A22,A23,
        A24,A25,A26,A27,A28,A29,A30,A31,
    ],
    output_types: [
        B0,B1,B2,B3,B4,B5,B6,B7,B8,B9,B10,B11,B12,
        B13,B14,B15,B16,B17,B18,B19,B20,B21,B22,B23,
        B24,B25,B26,B27,B28,B29,B30,B31,
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

#[test]
fn test_join() {
    use crate::{block_on, task};
    const OK: Result<(), ()> = Ok(());
    const ERR: Result<(), ()> = Err(());
    block_on(async {
        try_join!(
            async {
                task::yield_now().await;
                task::yield_now().await;
                OK
            },
            async { OK }
        )
        .ok();
        try_join!(async { OK }, async {
            task::yield_now().await;
            task::yield_now().await;
            ERR
        },)
        .ok();
        try_join!(async { ERR }, async {
            task::yield_now().await;
            task::yield_now().await;
            OK
        },)
        .ok();
    })
    .unwrap();
}
