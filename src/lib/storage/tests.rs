use std::env;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use itertools::Itertools;
use rand::prelude::*;
use rstest::rstest;
use tempfile::{tempdir, tempfile, NamedTempFile};
use testcontainers::images::generic::{GenericImage, WaitFor};
use testcontainers::{clients, Docker, RunArgs};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::storage::providers::{FSStorage, S3StorageBuilder};
use crate::tests::*;

use super::transaction::*;

fn setup_memory_bytestream() -> ByteStream {
    let data = vec![1, 2, 3, 4, 5];
    ByteStream::from(data)
}

fn setup_unnamedfile_bytestream() -> ByteStream {
    let mut file = tempfile().expect("unable to create temp file");
    assert_eq!(file.write(&[1, 2, 3, 4, 5]).expect("write failed"), 5);
    file.seek(SeekFrom::Start(0)).expect("unable to rewind");
    ByteStream::from(file)
}

fn setup_tempfile_bytestream() -> ByteStream {
    let mut file = NamedTempFile::new().expect("unable to create temp file");
    assert_eq!(file.write(&[1, 2, 3, 4, 5]).expect("write failed"), 5);
    file.seek(SeekFrom::Start(0)).expect("unable to rewind");
    ByteStream::from(file)
}

fn setup_pathfile_bytestream() -> ByteStream {
    let mut file = std::fs::File::create("tests/stream.test").expect("unable to create file");
    assert_eq!(file.write(&[1, 2, 3, 4, 5]).expect("write failed"), 5);
    file.seek(SeekFrom::Start(0)).expect("unable to rewind");
    ByteStream::from_path("tests/stream.test").expect("unable to create stream")
}

#[rstest]
#[case(setup_memory_bytestream())]
#[case(setup_unnamedfile_bytestream())]
#[case(setup_tempfile_bytestream())]
#[case(setup_pathfile_bytestream())]
#[tokio::test]
async fn test_bytestream_read(#[case] mut stream: ByteStream) {
    let mut read_buf = vec![];
    stream
        .read_to_end(&mut read_buf)
        .await
        .expect("read failed");
    assert_eq!(read_buf, [1, 2, 3, 4, 5], "content mismatch");

    stream.seek(SeekFrom::Start(1)).await.expect("seek failed");
    let mut read_buf = vec![];
    stream
        .read_to_end(&mut read_buf)
        .await
        .expect("read failed");
    assert_eq!(read_buf, [2, 3, 4, 5], "content mismatch");
}

#[rstest]
#[case(setup_memory_bytestream())]
#[case(setup_tempfile_bytestream())]
#[case(setup_pathfile_bytestream())]
#[tokio::test]
async fn test_bytestream_clone(#[case] mut stream: ByteStream) {
    let mut read_buf = vec![];
    stream
        .read_to_end(&mut read_buf)
        .await
        .expect("read failed");
    assert_eq!(read_buf, [1, 2, 3, 4, 5], "content mismatch");

    let mut cloned_stream: ByteStream = stream.clone();
    let mut read_buf = vec![];
    cloned_stream
        .read_to_end(&mut read_buf)
        .await
        .expect("read failed");
    assert_eq!(read_buf, [1, 2, 3, 4, 5], "content mismatch");
}

#[rstest]
#[case(setup_memory_bytestream(), PathBuf::from("tests/persist.test.1"))] // in-memory stream
#[case(setup_unnamedfile_bytestream(), PathBuf::from("tests/persist.test.2"))] // bare file stream
#[case(setup_tempfile_bytestream(), PathBuf::from("tests/persist.test.3"))] // namedfile to different fs (on my pc)
#[case(setup_tempfile_bytestream(), env::temp_dir().join("archer_persist.test"))] // namedfile to same fs
#[case(setup_pathfile_bytestream(), PathBuf::from("tests/persist.test.4"))] // path backed file
#[tokio::test]
async fn test_bytestream_persist(#[case] stream: ByteStream, #[case] persist_path: PathBuf) {
    drop(std::fs::remove_file(&persist_path));
    stream
        .into_file(&persist_path)
        .await
        .expect("unable to persist to file");
    let data = std::fs::read(&persist_path).expect("unable to read file");
    assert_eq!(data, [1, 2, 3, 4, 5], "content mismatch");
    std::fs::remove_file(persist_path).expect("cleanup failed");
}

async fn must_provider_work(storage: impl StorageProvider, strict: bool) {
    storage
        .put_file("test-1".as_ref(), vec![1, 2, 3, 4, 5].into())
        .await
        .expect("put failed");
    storage
        .put_file("test-2".as_ref(), vec![1, 2, 3, 4, 5, 6].into())
        .await
        .expect("put failed");

    if strict {
        assert!(
            matches!(
                storage
                    .delete_file("invalid-file".as_ref())
                    .await
                    .unwrap_err(),
                StorageError::FileNotExists(_)
            ),
            "deleting invalid file"
        );
    }
    assert!(
        matches!(
            storage.get_file("invalid-file".as_ref()).await.unwrap_err(),
            StorageError::FileNotExists(_)
        ),
        "getting invalid file"
    );

    let mut stream_1 = storage
        .get_file("test-1".as_ref())
        .await
        .expect("get failed");
    assert!(stream_1.in_memory());
    let mut read_buf = vec![];
    stream_1
        .read_to_end(&mut read_buf)
        .await
        .expect("read failed");
    assert_eq!(read_buf, [1, 2, 3, 4, 5], "content mismatch");

    let mut stream_2 = storage
        .get_file("test-2".as_ref())
        .await
        .expect("get failed");
    assert!(!stream_2.in_memory());
    let mut read_buf = vec![];
    stream_2
        .read_to_end(&mut read_buf)
        .await
        .expect("read failed");
    assert_eq!(read_buf, [1, 2, 3, 4, 5, 6], "content mismatch");

    storage
        .delete_file("test-2".as_ref())
        .await
        .expect("delete failed");
    assert!(
        matches!(
            storage.get_file("test-2".as_ref()).await.unwrap_err(),
            StorageError::FileNotExists(_)
        ),
        "getting deleted file"
    );
}

#[tokio::test]
async fn test_fs_provider() {
    let test_dir = tempdir().expect("temp dir creation failed");
    let fs_storage = FSStorage::new_with_limit(test_dir.path(), 5);

    must_provider_work(fs_storage, true).await
}

#[tokio::test]
async fn test_s3_provider() {
    let s3_storage = S3StorageBuilder::new()
        .with_name("mock-s3")
        .with_bucket("test-bucket")
        .with_credential("", "")
        .with_memory_limit(5);

    if let Some(endpoint) = option_env!("S3_ENDPOINT") {
        let s3_storage = s3_storage
            .with_endpoint(endpoint)
            .build()
            .expect("unable to build s3 storage");

        must_provider_work(s3_storage, false).await
    } else {
        let client = Arc::new(clients::Cli::default());
        let image = GenericImage::new("adobe/s3mock")
            .with_env_var("initialBuckets", "test-bucket")
            .with_wait_for(WaitFor::message_on_stdout("Started S3MockApplication"));
        let args = RunArgs::default().with_mapped_port((9090, 9090));
        let _container = client.run_with_args(image, args);

        let s3_storage = s3_storage
            .with_endpoint("http://localhost:9090")
            .build()
            .expect("unable to build s3 storage");

        must_provider_work(s3_storage, false).await
    }
}

#[derive(Default)]
struct MockProvider {
    seq: Mutex<Vec<TxnAction>>,
}

impl MockProvider {
    fn assert_ord(&self, path_1: &Path, path_2: &Path) {
        let pos_1 = self
            .seq
            .lock()
            .unwrap()
            .iter()
            .find_position(|a| match a {
                TxnAction::Put(p, _) => p == path_1,
                TxnAction::Delete(p) => p == path_1,
                TxnAction::Barrier => unreachable!(),
            })
            .unwrap()
            .0;
        let pos_2 = self
            .seq
            .lock()
            .unwrap()
            .iter()
            .find_position(|a| match a {
                TxnAction::Put(p, _) => p == path_2,
                TxnAction::Delete(p) => p == path_2,
                TxnAction::Barrier => unreachable!(),
            })
            .unwrap()
            .0;
        assert!(pos_1 < pos_2, "ord assertion failed");
    }
}

#[async_trait]
impl StorageProvider for MockProvider {
    async fn get_file(&self, _path: &Path) -> Result<ByteStream> {
        panic!("get_file not supported")
    }

    async fn put_file(&self, path: &Path, data: ByteStream) -> Result<()> {
        tokio::time::sleep(Duration::from_millis((random::<f32>() * 50.) as u64)).await;
        self.seq
            .lock()
            .unwrap()
            .push(TxnAction::Put(path.to_path_buf(), data));
        tokio::time::sleep(Duration::from_millis((random::<f32>() * 50.) as u64)).await;
        Ok(())
    }

    async fn delete_file(&self, path: &Path) -> Result<()> {
        tokio::time::sleep(Duration::from_millis((random::<f32>() * 20.) as u64)).await;
        self.seq
            .lock()
            .unwrap()
            .push(TxnAction::Delete(path.to_path_buf()));
        tokio::time::sleep(Duration::from_millis((random::<f32>() * 20.) as u64)).await;
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn must_txn() {
    let mut txn = Txn::new();
    txn.add(TxnAction::Put("1".into(), setup_memory_bytestream()));
    txn.add(TxnAction::Put("2".into(), setup_memory_bytestream()));
    txn.add(TxnAction::Put("3".into(), setup_memory_bytestream()));
    txn.add(TxnAction::Delete("4".into()));
    txn.add(TxnAction::Put("5".into(), setup_memory_bytestream()));
    txn.add(TxnAction::Put("6".into(), setup_memory_bytestream()));
    txn.add(TxnAction::Barrier);
    txn.add(TxnAction::Delete("7".into()));
    txn.add(TxnAction::Put("8".into(), setup_memory_bytestream()));
    txn.add(TxnAction::Delete("9".into()));
    txn.add(TxnAction::Put("10".into(), setup_memory_bytestream()));
    txn.add(TxnAction::Delete("11".into()));
    txn.add(TxnAction::Put("12".into(), setup_memory_bytestream()));
    txn.add(TxnAction::Delete("13".into()));
    txn.add(TxnAction::Barrier);
    txn.add(TxnAction::Delete("14".into()));
    txn.add(TxnAction::Delete("15".into()));

    let mock_provider = MockProvider::default();
    txn.commit(&mock_provider).await.expect("unable to commit");

    let ord_1 = (1..=6).cartesian_product(7..=13);
    ord_1.into_iter().for_each(|(x, y)| {
        mock_provider.assert_ord(&PathBuf::from(x.to_string()), &PathBuf::from(y.to_string()))
    });
    let ord_2 = (7..=13).cartesian_product(14..=15);
    ord_2.into_iter().for_each(|(x, y)| {
        mock_provider.assert_ord(&PathBuf::from(x.to_string()), &PathBuf::from(y.to_string()))
    });
}
