//! This is used for internal macros only.
//! Changes to this API are not considered breaking.

use std::io;
use std::panic::UnwindSafe;
use std::process::{ExitCode, Termination};

mod sealed {
    pub trait Sealed {}
    impl Sealed for bool {}
    impl Sealed for usize {}
}
pub trait IntoScale: sealed::Sealed {
    fn scale(self) -> usize;
}

impl IntoScale for bool {
    fn scale(self) -> usize {
        if self {
            affinity::get_core_num()
        } else {
            1
        }
    }
}

impl IntoScale for usize {
    fn scale(self) -> usize {
        self
    }
}

pub fn run<T>(scale: impl IntoScale, restart: bool, main: fn() -> io::Result<T>) -> ExitCode
where
    T: Termination,
{
    let scale = scale.scale();
    if scale == 1 && !restart {
        main().unwrap().report()
    } else if scale == 1 {
        no_scale_restart(main)
    } else if !restart {
        scaled_no_restart(scale, main)
    } else {
        scaled_and_restart(scale, || main().report())
    }
}

fn no_scale_restart<T: Termination>(main: fn() -> io::Result<T>) -> ExitCode {
    loop {
        match std::panic::catch_unwind(main) {
            Ok(ok) => return ok.unwrap().report(),
            Err(_) => {
                eprintln!("osiris: restarting thread");
                continue;
            }
        }
    }
}

fn scaled_no_restart<T: Termination>(scale: usize, main: fn() -> io::Result<T>) -> ExitCode {
    let n = affinity::get_core_num();
    std::thread::scope(|s| {
        for id in 0..scale {
            let id = id % n;
            s.spawn(move || {
                affinity::set_thread_affinity([id]).ok();
                main().unwrap().report();
            });
        }
    });
    ExitCode::SUCCESS
}

fn scaled_and_restart(
    scale: usize,
    main: impl Fn() -> ExitCode + Copy + Clone + Sync + Send + UnwindSafe,
) -> ExitCode {
    std::thread::scope(|s| {
        let n = affinity::get_core_num();
        let (tx, rx) = std::sync::mpsc::channel();

        for id in 0..scale {
            let tx = tx.clone();
            s.spawn(move || {
                let id = id % n;
                affinity::set_thread_affinity([id]).ok();
                tx.send((id, std::panic::catch_unwind(main)))
            });
        }

        let mut exit_count = 0;

        while exit_count < scale {
            let Ok((id, res)) = rx.recv() else {
                unreachable!();
            };
            let Err(_) = res else {
                exit_count += 1;
                continue;
            };
            // we restart the panicked dead replica
            let tx = tx.clone();

            s.spawn(move || {
                eprintln!("osiris: restarting thread #{id}");
                affinity::set_thread_affinity([id]).ok();
                tx.send((id, std::panic::catch_unwind(main)))
            });
        }
        ExitCode::SUCCESS
    })
}
