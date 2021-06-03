use std::io::Result as IOResult;
use std::io::{Cursor, SeekFrom};
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures::{ready, Stream};
use tempfile::NamedTempFile;
use tokio::io::{AsyncRead, AsyncSeek, AsyncSeekExt, AsyncWriteExt, ReadBuf};

use crate::utils::is_same_fs;

#[derive(Debug)]
pub enum ByteStream {
    Memory(Cursor<Vec<u8>>),
    File {
        file: tokio::fs::File,
        temp_file: Option<Arc<NamedTempFile>>,
        length: u64,
    },
}

impl ByteStream {
    pub fn in_memory(&self) -> bool {
        matches!(self, ByteStream::Memory(_))
    }

    pub fn size(&self) -> u64 {
        match self {
            ByteStream::Memory(v) => v.get_ref().len() as u64,
            ByteStream::File { length, .. } => *length,
        }
    }

    pub async fn into_file(self, path: impl AsRef<Path> + Clone) -> IOResult<()> {
        use tokio::fs::File;
        match self {
            ByteStream::Memory(v) => {
                let data = v.into_inner();
                let mut dest = File::create(path).await?;
                dest.write_all(&data).await?;
                dest.flush().await?;
            }
            ByteStream::File {
                temp_file: Some(file),
                ..
            } => {
                if is_same_fs(file.path(), path.clone()) {
                    match Arc::try_unwrap(file) {
                        Ok(file) => {
                            // this stream is the only owner of the file, persist
                            file.persist(path)?;
                        }
                        Err(file) => {
                            // this stream isn't the only owner, copy file
                            tokio::fs::copy(file.path(), path).await?;
                        }
                    }
                } else {
                    // we can't persist tempfile across filesystems
                    tokio::fs::copy(file.path(), path).await?;
                }
            }
            ByteStream::File {
                temp_file: None,
                mut file,
                ..
            } => {
                file.seek(SeekFrom::Start(0)).await?;
                let mut dest = File::create(path).await?;
                tokio::io::copy(&mut file, &mut dest).await?;
            }
        }
        Ok(())
    }
}

impl Clone for ByteStream {
    // NOTE
    // the cloned bytestream will have its pointer rewound
    fn clone(&self) -> Self {
        match self {
            ByteStream::Memory(v) => ByteStream::Memory(Cursor::new(v.clone().into_inner())), // TODO use custom cursor to avoid this clone
            ByteStream::File {
                temp_file: Some(temp_file),
                length,
                ..
            } => ByteStream::File {
                file: tokio::fs::File::from_std(temp_file.reopen().unwrap()),
                temp_file: Some(temp_file.clone()),
                length: *length,
            },
            ByteStream::File {
                temp_file: None, ..
            } => {
                // NOTE
                // It's possible to support cloning arbitrary file-backed bytestream by
                // 1. create another handle on the same fd
                // 2. record its current pos (by seek(current))
                // 3. rewind it
                // 4. copy it to another temp file
                // 5. create the new stream on the newly created temp file
                // 6. seek the original file to the previously saved pos
                // However, it's unsafe and won't sync well.
                panic!("unsupported")
            }
        }
    }
}

impl Stream for ByteStream {
    type Item = IOResult<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut inner_buf = [0; 8192];
        let mut buf = ReadBuf::new(&mut inner_buf);
        match ready!(self.poll_read(cx, &mut buf)) {
            Ok(_) => Some(Ok(Bytes::from(Vec::from(buf.filled())))).into(),
            Err(e) => Some(Err(e)).into(),
        }
    }
}

impl From<Vec<u8>> for ByteStream {
    fn from(v: Vec<u8>) -> Self {
        Self::Memory(Cursor::new(v))
    }
}

impl From<NamedTempFile> for ByteStream {
    fn from(f: NamedTempFile) -> Self {
        let length = f.as_file().metadata().unwrap().len();
        Self::File {
            file: f.reopen().unwrap().into(),
            temp_file: Some(Arc::new(f)),
            length,
        }
    }
}

impl From<std::fs::File> for ByteStream {
    fn from(f: std::fs::File) -> Self {
        let length = f.metadata().unwrap().len();
        Self::File {
            file: f.into(),
            temp_file: None,
            length,
        }
    }
}

impl AsyncRead for ByteStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<IOResult<()>> {
        match self.get_mut() {
            ByteStream::Memory(v) => Pin::new(v).poll_read(cx, buf),
            ByteStream::File { file: f, .. } => Pin::new(f).poll_read(cx, buf),
        }
    }
}

impl AsyncSeek for ByteStream {
    fn start_seek(self: Pin<&mut Self>, position: SeekFrom) -> IOResult<()> {
        match self.get_mut() {
            ByteStream::Memory(v) => Pin::new(v).start_seek(position),
            ByteStream::File { file: f, .. } => Pin::new(f).start_seek(position),
        }
    }

    fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IOResult<u64>> {
        match self.get_mut() {
            ByteStream::Memory(v) => Pin::new(v).poll_complete(cx),
            ByteStream::File { file: f, .. } => Pin::new(f).poll_complete(cx),
        }
    }
}
