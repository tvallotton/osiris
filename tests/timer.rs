use std::time::Duration;

use osiris::block_on;
use osiris::time::sleep;
#[test]
fn timer_smoke_test() {
    block_on(async {
        let time = std::time::Instant::now();
        sleep(Duration::from_millis(1621)).await;
        println!("{:?}", time.elapsed())
    })
    .unwrap();
}
