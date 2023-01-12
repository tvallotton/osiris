#[cfg(target_os = "linux")]
use osiris::{block_on, time::sleep};
#[cfg(target_os = "linux")]
use std::time::Duration;
#[cfg(target_os = "linux")]
#[test]
fn timer_smoke_test() {
    block_on(async {
        let time = std::time::Instant::now();
        let dur = Duration::from_millis(1620);
        sleep(dur).await;
        assert!(time.elapsed() > dur);
    })
    .unwrap();
}
