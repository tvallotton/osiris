use osiris::time::sleep;
use std::time::Duration;

#[osiris::test]
pub async fn spawn_blocking() {
    let time = std::time::Instant::now();
    let task = osiris::task::spawn_blocking(|| {
        println!("asd");
        // panic!("asd");
        // std::thread::sleep(Duration::from_secs(1));
    });

    sleep(Duration::from_secs(1)).await;

    dbg!(time.elapsed());
    assert!(time.elapsed().as_secs() < 2);
}
