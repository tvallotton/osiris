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
            core_affinity::get_core_ids().unwrap_or(vec![]).len().max(1)
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
    let cores = &core_affinity::get_core_ids().unwrap_or(vec![]);
    let n = cores.len().max(1);
    std::thread::scope(|s| {
        for thread in 0..scale {
            s.spawn(move || {
                let core_id = cores.get(thread % n);
                if let Some(core_id) = core_id {
                    core_affinity::set_for_current(*core_id);
                }
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
    let cores = &core_affinity::get_core_ids().unwrap_or(vec![]);
    std::thread::scope(|s| {
        let (tx, rx) = std::sync::mpsc::channel();

        let n = cores.len().min(1);

        for thread in 0..scale {
            let tx = tx.clone();
            let core_id = cores.get(thread % n);
            s.spawn(move || {
                if let Some(core_id) = core_id {
                    core_affinity::set_for_current(*core_id);
                }
                tx.send((thread, std::panic::catch_unwind(main)))
            });
        }

        let mut exit_count = 0;

        while exit_count < scale {
            let Ok((thread, res)) = rx.recv() else {
                unreachable!();
            };
            let Err(_) = res else {
                exit_count += 1;
                continue;
            };
            // we restart the panicked dead replica
            let tx = tx.clone();
            let core_id = cores.get(thread % n);

            s.spawn(move || {
                eprintln!("osiris: restarting thread #{thread}");
                if let Some(core_id) = core_id {
                    core_affinity::set_for_current(*core_id);
                }
                tx.send((thread, std::panic::catch_unwind(main)))
            });
        }
        ExitCode::SUCCESS
    })
}
