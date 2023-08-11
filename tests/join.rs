use osiris::time::sleep;
use std::time::{Duration, Instant};

#[osiris::test]
async fn smoke_test() {
    let (a, b) = osiris::join!(async { 1 }, async { 2 });
    assert_eq!(a, 1);
    assert_eq!(b, 2);
}

#[osiris::test]
async fn test_waker() {
    let time = Instant::now();
    let dur = Duration::from_millis(50);

    osiris::join!(sleep(dur), sleep(dur), sleep(dur), sleep(dur));

    let elapsed = time.elapsed().as_secs_f64();
    let dur = dur.as_secs_f64();
    let approx_one = elapsed / dur;

    assert!(approx_one < 1.5, "{approx_one}");
    assert!(1.0 < approx_one, "{approx_one}");
}
