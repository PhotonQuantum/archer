use std::fs::File;
use std::io::Result as IOResult;
use std::io::{Cursor, SeekFrom};
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use derive_more::From;
use futures::{ready, Stream};
use tokio::io::{AsyncRead, AsyncSeek, ReadBuf};

#[derive(Debug, From)]
pub enum ByteStream {
    Memory(Cursor<Vec<u8>>),
    File { file: tokio::fs::File, length: u64 },
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

impl From<std::fs::File> for ByteStream {
    fn from(f: File) -> Self {
        let length = f.metadata().unwrap().len();
        Self::File {
            file: f.into(),
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
