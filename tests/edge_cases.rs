use osiris::runtime::block_on;
use osiris::task::{spawn, yield_now};

/// This tests makes sure a task can spawn other tasks on abort
#[test]
fn spawn_on_abort() {
    struct SpawnOnDrop;

    impl Drop for SpawnOnDrop {
        fn drop(&mut self) {
            spawn(async {
                yield_now().await;
            });
        }
    }

    block_on(async move {
        let handle = spawn(async {
            let _span_on_drop = SpawnOnDrop;
            yield_now().await;
        });
        yield_now().await;
        handle.abort();
        for _ in 0..64 {
            yield_now().await;
        }
    })
    .unwrap();
}

#[test]
fn child_panic_is_catched() {
    block_on(async {
        spawn(async {
            spawn(async { panic!("child panic") }).await;
        })
        .await;
        println!("asd");
        yield_now().await;
        yield_now().await;
    })
    .unwrap();
}
