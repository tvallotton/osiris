use osiris::time::sleep;
use std::time::Duration;

#[osiris::test]
pub async fn spawn_blocking_doesnt_block() {
    let time = std::time::Instant::now();
    let task = osiris::task::spawn_blocking(|| {
        std::thread::sleep(Duration::from_secs(1));
        234523
    });

    sleep(Duration::from_millis(250)).await;

    dbg!(time.elapsed());
    assert_eq!(task.await, 234523);
    assert!(time.elapsed().as_millis() < 1250);
}
