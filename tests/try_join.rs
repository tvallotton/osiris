use osiris::task::yield_now;
use osiris::time::sleep;
use osiris::try_join;
use std::time::{Duration, Instant};

#[osiris::test]
async fn smoke_test() {
    let (a, b) = try_join!(async { std::io::Result::Ok(1) }, async { Ok(2) }).unwrap();
    assert_eq!(a, 1);
    assert_eq!(b, 2);

    let err = try_join!(async { Ok(true) }, async {
        Result::<i32, &str>::Err("foo")
    })
    .unwrap_err();

    assert_eq!(err, "foo");
}

async fn ok_fn(dur: Duration) -> Result<(), &'static str> {
    sleep(dur).await;
    Ok(())
}

async fn err_fn() -> Result<(), &'static str> {
    yield_now().await;
    Err("error")
}

#[osiris::test]
async fn test_waker_ok() {
    let time = Instant::now();
    let dur = Duration::from_millis(50);

    try_join!(
        ok_fn(dur),
        ok_fn(dur),
        ok_fn(dur),
        ok_fn(dur),
        ok_fn(dur),
        ok_fn(dur)
    )
    .unwrap();

    let elapsed = time.elapsed().as_secs_f64();
    let dur = dur.as_secs_f64();
    let approx_one = elapsed / dur;

    assert!(approx_one < 1.5, "{approx_one}");
    assert!(1.0 < approx_one, "{approx_one}");
}

#[osiris::test]
async fn test_waker_err() {
    let time = Instant::now();
    let dur = Duration::from_millis(50);

    try_join!(
        ok_fn(dur),
        ok_fn(dur),
        ok_fn(dur),
        err_fn(),
        ok_fn(dur),
        ok_fn(dur),
    )
    .unwrap_err();

    let elapsed = time.elapsed().as_secs_f64();
    let dur = dur.as_secs_f64();
    let approx_one = elapsed / dur;

    assert!(approx_one < 0.01, "{approx_one}");
}
