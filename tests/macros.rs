use std::sync::Mutex;

#[test]
fn test_plain_main() {
    static COUNT: Mutex<i32> = Mutex::new(0);

    #[osiris::main]
    async fn plain_main() {
        *COUNT.lock().unwrap() += 1;
    }

    plain_main();
    assert!(*COUNT.lock().unwrap() == 1);
}

#[test]
fn test_scaled_main() {
    static COUNT: Mutex<i32> = Mutex::new(0);

    #[osiris::main(scale = true)]
    async fn scaled_main() {
        *COUNT.lock().unwrap() += 1;
    }

    scaled_main();
    assert!(*COUNT.lock().unwrap() > 1);
}

#[test]
fn test_main_with_restart() {
    static COUNT: Mutex<i32> = Mutex::new(0);

    #[osiris::main(restart = true)]
    async fn main_with_restart() {
        let mut guard = COUNT.lock().unwrap();
        if *guard == 0 {
            *guard += 1;
            drop(guard);
            panic!()
        }
        *guard += 1;
    }
    main_with_restart();
    assert!(*COUNT.lock().unwrap() == 2);
}

#[test]
fn test_main_that_panics_once() {
    static PANICKED: Mutex<bool> = Mutex::new(false);
    static COUNT: Mutex<i32> = Mutex::new(0);

    #[osiris::main(scale = 2, restart = true)]
    async fn main_that_panics_once() {
        let mut panicked = PANICKED.lock().unwrap();
        if !*panicked {
            *panicked = true;
            drop(panicked);
            panic!()
        }
        *COUNT.lock().unwrap() += 1;
    }

    main_that_panics_once();

    assert!(*COUNT.lock().unwrap() == 2)
}
