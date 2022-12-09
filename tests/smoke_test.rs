use std::cell::Cell;

use osiris::runtime::block_on;
use osiris::task::{self, spawn, yield_now};

#[test]
fn smoke_test() {
    block_on(async {
        let value = spawn(async { 10 }).await;
        assert_eq!(value, 10);
    })
    .unwrap();
}

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
        spawn(async {
            yield_now().await;
            1
        })
        .await;
        joined = true;
    })
    .unwrap();
    assert!(joined);
}
