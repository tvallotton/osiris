#[cfg(target_os = "linux")]
use osiris::{
    block_on,
    fs::{File, OpenOptions},
    time::Duration,
};

#[cfg(target_os = "linux")]
#[test]
fn open_file() {
    use std::io;

    block_on(async { osiris::time::sleep(Duration::from_secs(1)).await }).unwrap();
}
