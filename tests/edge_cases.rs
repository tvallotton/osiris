use core::num;
use std::cell::Cell;
use std::panic::catch_unwind;

use osiris::runtime::block_on;
use osiris::task::{spawn, yield_now};

fn install() {
    #[cfg(not(miri))]
    {
        dotenv::dotenv().ok();
    }
}

/// This tests makes sure a task can spawn other tasks on abort
#[test]
fn spawn_on_abort() {
    struct SpawnOnDrop;

    impl Drop for SpawnOnDrop {
        fn drop(&mut self) {
            spawn(async {
                yield_now().await;
                SUCCESS.with(|val| val.set(true));
            })
            .detach();
        }
    }

    thread_local! {
        static SUCCESS: Cell<bool> = Cell::default();
    }

    block_on(async move {
        let handle = spawn(async {
            let _span_on_drop = SpawnOnDrop;
            loop {
                yield_now().await;
            }
        });
        yield_now().await;
        handle.abort();
        for _ in 0..64 {
            yield_now().await;
        }
    })
    .unwrap();
    // make sure the spawned task runned.
    assert!(SUCCESS.with(|val| val.get()))
}

// this function tests that panics are propagated across join handles.
#[test]
fn propagate_panic() {
    install();
    let result = catch_unwind(|| {
        block_on(async {
            // joined JoinHandle propagates
            spawn(async {
                // dropped JoinHandle propagates
                spawn(async { panic!("child panic") }).await;
            })
            .await;
        })
        .unwrap()
    });
    assert!(result.is_err());
}

// this function tests that panics aren't propagated across detached join handles.
#[test]
fn detach_handle_panic() {
    install();

    // test for child tasks
    block_on(async {
        spawn(async {
            let mut handle = spawn(async { panic!("child panic") });
            handle.detach();
            yield_now().await;
            yield_now().await;
        })
        .await;
        yield_now().await;
        yield_now().await;
    })
    .unwrap();
    // test for main task
    block_on(async {
        let mut handle = spawn(async {
            panic!("child panic");
        });
        handle.detach();
        yield_now().await;
    })
    .unwrap();
}

// this test makes sure a task can abort itself
fn self_abort() {
    todo!()
}

#[test]
fn self_join() {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(async {
            let (tx, rx) = tokio::sync::oneshot::channel();

            let h = tokio::spawn(async move {
                let h = rx.await.unwrap();
                println!("handler: {h:?}");
                let number = h.await;
                println!("number: {number:?}");
                10
            });
            tx.send(h).unwrap();
            for _ in 0..1000 {
                yield_now().await;
            }
        });
}
