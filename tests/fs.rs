use osiris::block_on;
use osiris::fs::{create_dir, metadata, remove_dir};

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

#[test]
fn fooooo() {
    block_on(async {
        let x = remove_dir("tests/fs_test_files/non_empty_dir").await;
        x.unwrap();
    })
    .unwrap();
}
