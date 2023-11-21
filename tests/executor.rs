use osiris::detach;
use osiris::runtime::block_on;
use osiris::task::{spawn, yield_now};
use std::cell::Cell;
use std::panic::catch_unwind;
use std::rc::Rc;

use osiris::task::{self};

fn install() {
    #[cfg(not(miri))]
    {
        dotenv::dotenv().ok();
    }
}

async fn stall() {
    for _ in 0..10 {
        yield_now().await;
    }
}

#[test]
fn test_spawn() {
    let spawned = Rc::new(Cell::new(false));

    block_on(async {
        let spawned = spawned.clone();
        let value = spawn(async move {
            spawned.set(true);
            10
        })
        .await;
        assert_eq!(value, 10);
    })
    .unwrap();
    assert!(spawned.get());
}
// This thest makes sure the task id function returns different values for different tasks
#[test]
fn unique_task_id() {
    block_on(async {
        let task_id = task::id();
        spawn(async move {
            assert_ne!(task_id, task::id());
        })
        .await;
    })
    .unwrap();
}
/// This test makes sure task can be spawned, and they are joined.
#[test]
fn spawn_can_be_joined() {
    let mut joined = false;
    block_on(async {
        let number = spawn(async {
            yield_now().await;
            1
        })
        .await;
        assert_eq!(number, 1);
        joined = true;
    })
    .unwrap();
    assert!(joined);
}

/// This tests makes sure a task can spawn other tasks on abort
#[test]
fn spawn_on_abort() {
    struct SpawnOnDrop;

    impl Drop for SpawnOnDrop {
        fn drop(&mut self) {
            detach(async {
                yield_now().await;
                SUCCESS.with(|val| val.set(true));
            });
        }
    }

    thread_local! {
        static SUCCESS: Cell<bool> = Cell::default();
    }

    block_on(async move {
        let handle = spawn(async {
            let _span_on_drop = SpawnOnDrop;
            yield_now().await;
        });
        stall().await;
        handle.abort();
        stall().await;
    })
    .unwrap();
    // make sure the spawned task runned.
    assert!(SUCCESS.with(|val| val.get()))
}

// this function tests that panics are propagated when joining join handles.
#[test]
fn joining_join_handle_propagates_panics() {
    install();
    let result = catch_unwind(|| {
        block_on(async {
            spawn(async {
                spawn(async { panic!("child panic") }).await;
            })
            .await;
        })
        .unwrap();
    });
    assert!(result.is_err());
}

// this function tests that panics are propagated when dropping join handles
#[test]
fn dropped_join_handle_propagates_panics() {
    install();
    let result = catch_unwind(|| {
        block_on(async {
            let h = spawn(async {
                let h = spawn(async { panic!("child panic") });
                stall().await;
                drop(h);
            });
            stall().await;
            drop(h);
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
            let handle = detach(async { panic!("child panic") });

            stall().await;
        })
        .await;
        detach(async { panic!() });
        stall().await;
    })
    .unwrap();
    // test for main task
    block_on(async {
        let handle = detach(async {
            panic!("child panic");
        });

        stall().await;
    })
    .unwrap();
}

#[osiris::test]
async fn task_catch_unwind() {
    let res = spawn(async { panic!() }).catch_unwind().await;
    assert!(res.is_err());
}

// this function test that a function cannot abort itself
// currently the behavior is that it won't propagate to any other task
// so the program will continue as normal.
#[test]
fn self_abort() {
    block_on(async {
        let handle = Rc::new(Cell::new(None));
        let join_handle = spawn({
            let handle = handle.clone();
            async move {
                // panics!
                drop(handle.take());
            }
        });
        handle.set(Some(join_handle));
        stall().await
    })
    .unwrap();
}
