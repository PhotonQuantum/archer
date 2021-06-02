use std::io::{Seek, SeekFrom, Write};
use std::sync::Arc;

use rstest::rstest;
use tempfile::{tempdir, tempfile};
use testcontainers::images::generic::{GenericImage, WaitFor};
use testcontainers::{clients, Docker, RunArgs};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::storage::providers::{FSStorage, S3StorageBuilder};
use crate::tests::*;

fn setup_memory_bytestream() -> ByteStream {
    let data = vec![1, 2, 3, 4, 5];
    ByteStream::from(data)
}

fn setup_file_bytestream() -> ByteStream {
    let mut file = tempfile().expect("unable to create temp file");
    assert_eq!(file.write(&[1, 2, 3, 4, 5]).expect("write failed"), 5);
    file.seek(SeekFrom::Start(0)).expect("unable to rewind");
    ByteStream::from(file)
}

#[rstest]
#[case(setup_memory_bytestream())]
#[case(setup_file_bytestream())]
#[tokio::test]
async fn test_bytestream(#[case] mut stream: ByteStream) {
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
