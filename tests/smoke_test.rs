use osiris::runtime::block_on;
use osiris::task::{self, spawn};

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
