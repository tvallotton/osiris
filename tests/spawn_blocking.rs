use futures::FutureExt;
use osiris::time::sleep;
use std::panic::AssertUnwindSafe;
use std::time::Duration;

#[osiris::test]
pub async fn spawn_blocking_doesnt_block() {
    let time = std::time::Instant::now();
    let task = osiris::task::spawn_blocking(|| {
        std::thread::sleep(Duration::from_millis(100));
        234523
    });
    sleep(Duration::from_millis(50)).await;
    dbg!(time.elapsed());
    assert_eq!(task.await, 234523);
    assert!(time.elapsed().as_millis() < 150);
}

#[osiris::test]
pub async fn spawn_blocking_propagates_panic() {
    let task = osiris::task::spawn_blocking(|| panic!());
    assert!(AssertUnwindSafe(task).catch_unwind().await.is_err())
}
