use std::sync::atomic::{AtomicBool, Ordering};

#[osiris::main]
async fn plain_main() {}

#[osiris::main(scale = true)]
async fn scaled_main() {}

#[osiris::main(scale = true, restart = true)]
async fn main_with_restart() {}

static PANICKED: AtomicBool = AtomicBool::new(false);

#[osiris::main(scale = true, restart = true)]
async fn main_that_panics_once() {
    if !PANICKED.load(Ordering::Acquire) {
        PANICKED.store(true, Ordering::Release);
        panic!()
    }
}
#[test]
fn test_main_macro() {
    plain_main();
    scaled_main();
    main_with_restart();

    main_that_panics_once();
    // assert!(RESTARTED.with(|x| x.get()));
}
