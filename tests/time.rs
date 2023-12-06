use osiris::block_on;
use osiris::time::sleep;

use std::time::Duration;

#[test]
fn timer_smoke_test() {
    block_on(async {
        let time = std::time::Instant::now();
        sleep(Duration::from_millis(1234)).await;
        assert!(time.elapsed().as_millis() >= 1234, "{:?}", time.elapsed());
        dbg!(time.elapsed());
    })
    .unwrap();
}
