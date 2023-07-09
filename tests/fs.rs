#![cfg(target_os = "linux")]
use osiris::fs::{create_dir, metadata, remove_dir, remove_file, File, OpenOptions};

#[osiris::test]
async fn test_metadata() {
    let dir = metadata("tests/fs_test_files").await.unwrap();
    assert!(dir.is_dir());

    let bar = metadata("tests/fs_test_files/bar.txt").await.unwrap();
    assert!(bar.is_file());
    assert_eq!(bar.len(), 10);

    let _ = dbg!(bar.created());
    let _ = dbg!(bar.modified());
    let _ = dbg!(bar.accessed());
    let _ = dbg!(bar.is_symlink());
}

#[osiris::test]
async fn create_and_rm_dir() {
    let path = "tests/fs_test_files/new_dir";
    assert!(metadata(path).await.is_err());
    create_dir(path).await.unwrap();
    assert!(metadata(path).await.unwrap().is_dir());
    remove_dir(path).await.unwrap();
    assert!(metadata(path).await.is_err());
}

/// this test creates a file, writes to it with write_at,
/// closes it and opens it again to read it.
#[osiris::test]
async fn read_write_test() {
    let path = "tests/fs_test_files/read_write_test.txt";
    let file = File::create(path).await.unwrap();
    file.write_at("contents", 0).await.0.unwrap();
    file.close().await.unwrap();
    let file = File::open(path).await.unwrap();
    let buf = vec![0; 256];
    let (res, buf) = file.read_at(buf, 0).await;
    let len = res.unwrap();
    assert_eq!(&buf[..len], "contents".as_bytes());
    file.close().await.unwrap();
    remove_file(path).await.unwrap();
    assert!(metadata(path).await.is_err());
}

/// this test creates a file and writes to it twice using
/// `write` to make sure that the file position is advanced
/// properly
#[osiris::test]
async fn seekable_file_test() {
    let path = "tests/fs_test_files/seekable_file_test.txt";
    let file = File::create(path).await.unwrap();
    let written = file.write("hello ").await.0.unwrap();
    assert_eq!(written, 6);
    let written = file.write("world").await.0.unwrap();
    assert_eq!(written, 5);
    file.close().await.unwrap();
    let buf = vec![0; 11];
    let buf = File::open(path).await.unwrap().read(buf).await.1;
    assert_eq!(buf, b"hello world");
    remove_file(path).await.unwrap();
}

#[osiris::test]
async fn create_new() {
    let path = "tests/fs_test_files/create_new.txt";
    let file = File::create_new(path).await.unwrap();
    file.close().await.unwrap();
    File::create_new(path).await.err().unwrap();
    remove_file(path).await.unwrap();
}

#[osiris::test]
async fn test_permisions() {
    let path = "tests/fs_test_files/test_permisions.txt";
    File::create(path).await.unwrap().close().await.unwrap();
    let file = OpenOptions::new().read(true).open(path).await.unwrap();
    file.write("data").await.0.err().unwrap();
    remove_file(path).await.unwrap();
}

#[osiris::test]
async fn test_sync() {
    let path = "tests/fs_test_files/test_sync.txt";
    let file = File::create(path).await.unwrap();
    file.write("some data").await.0.unwrap();
    file.sync_data().await.unwrap();
    file.sync_all().await.unwrap();
    file.close().await.unwrap();
    remove_file(path).await.unwrap();
}
