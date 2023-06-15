use futures_lite::future::yield_now;
use osiris::block_on;
use osiris::fs::{create_dir, metadata, remove_dir, remove_file, File};

#[test]
fn test_metadata() {
    block_on(async {
        let dir = metadata("tests/fs_test_files").await.unwrap();
        assert!(dir.is_dir());

        let bar = metadata("tests/fs_test_files/bar.txt").await.unwrap();
        assert!(bar.is_file());
        assert_eq!(bar.len(), 10);

        let _ = dbg!(bar.created());
        let _ = dbg!(bar.modified());
        let _ = dbg!(bar.accessed());
    })
    .unwrap();
}

#[test]
fn create_and_rm_dir() {
    block_on(async {
        let path = "tests/fs_test_files/new_dir";
        assert!(metadata(path).await.is_err());
        create_dir(path).await.unwrap();
        assert!(metadata(path).await.unwrap().is_dir());
        remove_dir(path).await.unwrap();
        assert!(metadata(path).await.is_err());
    })
    .unwrap();
}

#[osiris::test]
async fn read_write_test() {
    let path = "tests/fs_test_files/new_file.txt";
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
