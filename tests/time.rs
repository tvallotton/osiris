use osiris::block_on;
use osiris::time::sleep;

use std::time::Duration;

#[test]
fn timer_smoke_test() {
    block_on(async {
        let time = std::time::Instant::now();
        sleep(Duration::from_millis(1620)).await;
        println!("{:?}", time.elapsed())
    })
    .unwrap();
}
