#[cfg(target_os = "linux")]
use osiris::{
    block_on,
    fs::{File, OpenOptions},
};
#[cfg(target_os = "linux")]
#[test]
fn open_file() {
    block_on(async { File::create("foo.txt").await }).unwrap();
}
