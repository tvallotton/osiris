//! This is used for internal macros only.
//! Changes to this API are not considered breaking.

use std::{process::{ExitCode, Termination}, panic::UnwindSafe};

pub fn start<T>(workers: u16, restart: bool, main: fn() -> T) -> ExitCode
where
    T: Termination,
{
    if workers == 1 && !restart {
        main().report()
    } else if workers == 1 {
        loop {
            match std::panic::catch_unwind(main) {
                Ok(ok) => return ok.report(),
                Err(_) => continue,
            }
        }
    } else if !restart {
        scaled_no_restart(workers, main)
    } else {
        scaled_and_restart(workers, || main().report())
    }
}

fn scaled_no_restart<T: Termination>(workers: u16,  main: fn() -> T) -> ExitCode {
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                main().report(); 
            });
        }
    });
    ExitCode::SUCCESS
}

fn scaled_and_restart(workers: u16,  main: impl Fn() -> ExitCode + UnwindSafe + Send + Copy) -> ExitCode {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::scope(|s| {
        for _ in 0..workers {
            let tx = tx.clone();
            s.spawn(move || tx.send(std::panic::catch_unwind(main)));
        }
        let mut exit_count = 0;
        loop {
            if exit_count >= workers {
                return std::process::ExitCode::SUCCESS;
            }
            let Ok(res) = rx.recv() else {
                unreachable!(); 
            };
            let Err(_) = res else {
                exit_count += 1;
                continue; 
            };
            // we restart the panicked dead replica
            let tx = tx.clone();
            s.spawn(move || tx.send(std::panic::catch_unwind(main)));
        }
    });
    ExitCode::SUCCESS
}
