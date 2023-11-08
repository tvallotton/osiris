use osiris::block_on;

#[osiris::test]
async fn thread_safe_send() {
    std::thread::scope(|s| {
        for _ in 0..10 {
            let (tx, mut rx) = tokio::sync::mpsc::channel(1);
            let number = 123;
            s.spawn(move || {
                block_on(async {
                    tx.send(number).await.unwrap();
                })
            });
            s.spawn(move || {
                block_on(async {
                    assert_eq!(rx.recv().await.unwrap(), number);
                })
            });
        }
    });
}
