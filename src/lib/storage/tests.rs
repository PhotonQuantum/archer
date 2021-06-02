use std::io::{Seek, SeekFrom, Write};

use rstest::rstest;
use tempfile::tempfile;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::storage::providers::FSStorage;
use crate::storage::types::ByteStream;

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
    assert_eq!(
        stream
            .read_to_end(&mut read_buf)
            .await
            .expect("read failed"),
        5,
        "length mismatch"
    );
    assert_eq!(read_buf, [1, 2, 3, 4, 5], "content mismatch");

    stream.seek(SeekFrom::Start(1)).await;
    let mut read_buf = vec![];
    assert_eq!(
        stream
            .read_to_end(&mut read_buf)
            .await
            .expect("read failed"),
        4,
        "length mismatch"
    );
    assert_eq!(read_buf, [2, 3, 4, 5], "content mismatch");
}
