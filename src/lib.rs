#![warn(clippy::undocumented_unsafe_blocks)]
#![warn(unsafe_op_in_unsafe_fn)]

use std::{
    future::{poll_fn, Future},
    task::{Context, Poll},
};

pub use runtime::block_on;
pub use task::spawn;

#[macro_use]
mod macros;

pub mod fs;
mod hasher;
pub mod io;
pub mod runtime;
pub mod shared_driver;
pub mod task;

#[test]
fn foo() {
    block_on(async {
        println!("hello world");
    })
    .unwrap();
}

pub async fn yield_now() {
    use std::future::Future;
    use std::pin::Pin;
    struct YieldNow {
        yielded: bool,
    };

    impl Unpin for YieldNow {}

    impl Future for YieldNow {
        type Output = ();
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
            cx.waker().wake_by_ref();
            if self.yielded {
                Poll::Ready(())
            } else {
                self.yielded = true;
                Poll::Pending
            }
        }
    }

    YieldNow { yielded: false }.await
}

pub async fn yield_now2() {
    let mut yielded = false;
    poll_fn(move |cx| {
        cx.waker().wake_by_ref();
        if yielded {
            Poll::Ready(())
        } else {
            yielded = true;
            Poll::Pending
        }
    })
    .await;
}
