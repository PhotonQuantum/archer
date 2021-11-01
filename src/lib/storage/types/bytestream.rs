use std::convert::TryFrom;
use std::fs::File;
use std::io::Result as IOResult;
use std::io::{Cursor, SeekFrom};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures::{ready, Stream};
use tempfile::NamedTempFile;
use tokio::io::{AsyncRead, AsyncSeek, AsyncSeekExt, AsyncWriteExt, ReadBuf};

use crate::utils::is_same_fs;

#[derive(Debug, Clone)]
pub enum FileObject {
    Unnamed,
    Path(PathBuf), // it's assumed that the file exists when object is alive
    NamedTemp(Arc<NamedTempFile>),
}

#[derive(Debug)]
pub enum ByteStream {
    Memory(Cursor<Vec<u8>>),
    File {
        handle: tokio::fs::File,
        object_type: FileObject,
        length: u64,
    },
}

impl ByteStream {
    pub fn from_path(path: impl AsRef<Path>) -> IOResult<Self> {
        let handle = std::fs::File::open(path.as_ref())?;
        let length = handle.metadata()?.len();
        Ok(Self::File {
            handle: tokio::fs::File::from_std(handle),
            object_type: FileObject::Path(path.as_ref().to_path_buf()),
            length,
        })
    }

    pub const fn in_memory(&self) -> bool {
        matches!(self, ByteStream::Memory(_))
    }

    pub fn size(&self) -> u64 {
        match self {
            ByteStream::Memory(v) => v.get_ref().len() as u64,
            ByteStream::File { length, .. } => *length,
        }
    }

    pub async fn into_file(self, path: impl AsRef<Path> + Clone + Send) -> IOResult<()> {
        use tokio::fs::File;
        match self {
            ByteStream::Memory(v) => {
                let data = v.into_inner();
                let mut dest = File::create(path).await?;
                dest.write_all(&data).await?;
                dest.sync_all().await?;
            }
            ByteStream::File {
                object_type: FileObject::NamedTemp(file),
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
                object_type: FileObject::Path(src_path),
                ..
            } => {
                tokio::fs::copy(src_path, path).await?;
            }
            ByteStream::File {
                handle: mut file, ..
            } => {
                file.seek(SeekFrom::Start(0)).await?;
                let mut dest = File::create(path).await?;
                tokio::io::copy(&mut file, &mut dest).await?;
                dest.sync_all().await?;
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
            ByteStream::Memory(v) => Self::Memory(Cursor::new(v.clone().into_inner())), // TODO use custom cursor to avoid this clone
            ByteStream::File {
                object_type: FileObject::NamedTemp(temp_file),
                length,
                ..
            } => Self::File {
                handle: tokio::fs::File::from_std(temp_file.reopen().unwrap()),
                object_type: FileObject::NamedTemp(temp_file.clone()),
                length: *length,
            },
            ByteStream::File {
                object_type: FileObject::Path(file_path),
                ..
            } => {
                let mut src = std::fs::File::open(file_path).unwrap();
                let mut new_file = NamedTempFile::new().unwrap();
                std::io::copy(&mut src, &mut new_file).unwrap();
                Self::try_from(new_file).unwrap()
            }
            ByteStream::File {
                object_type: FileObject::Unnamed,
                ..
            } => {
                // NOTE
                // It's possible to support cloning unnamed file backed bytestream by
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

impl TryFrom<NamedTempFile> for ByteStream {
    type Error = std::io::Error;

    fn try_from(f: NamedTempFile) -> Result<Self, Self::Error> {
        let length = f.as_file().metadata()?.len();
        Ok(Self::File {
            handle: f.reopen()?.into(),
            object_type: FileObject::NamedTemp(Arc::new(f)),
            length,
        })
    }
}

impl TryFrom<std::fs::File> for ByteStream {
    type Error = std::io::Error;

    fn try_from(f: File) -> Result<Self, Self::Error> {
        let length = f.metadata()?.len();
        Ok(Self::File {
            handle: f.into(),
            object_type: FileObject::Unnamed,
            length,
        })
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
            ByteStream::File { handle: f, .. } => Pin::new(f).poll_read(cx, buf),
        }
    }
}

impl AsyncSeek for ByteStream {
    fn start_seek(self: Pin<&mut Self>, position: SeekFrom) -> IOResult<()> {
        match self.get_mut() {
            ByteStream::Memory(v) => Pin::new(v).start_seek(position),
            ByteStream::File { handle: f, .. } => Pin::new(f).start_seek(position),
        }
    }

    fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IOResult<u64>> {
        match self.get_mut() {
            ByteStream::Memory(v) => Pin::new(v).poll_complete(cx),
            ByteStream::File { handle: f, .. } => Pin::new(f).poll_complete(cx),
        }
    }
}
