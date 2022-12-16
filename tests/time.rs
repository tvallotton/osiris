#[cfg(target_os = "linux")]
use osiris::{block_on, time::sleep};
#[cfg(target_os = "linux")]
use std::time::Duration;
#[cfg(target_os = "linux")]
#[test]
fn timer_smoke_test() {
    use io_uring::IoUring;
    block_on(async {
        let time = std::time::Instant::now();
        sleep(Duration::from_millis(1620)).await;
        println!("{:?}", time.elapsed())
    })
    .unwrap();
}
