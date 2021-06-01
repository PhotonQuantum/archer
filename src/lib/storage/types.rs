use std::fs::File;
use std::io::Result as IOResult;
use std::io::{Cursor, Error, SeekFrom};
use std::pin::Pin;
use std::task::{Context, Poll};

use derive_more::From;
use tokio::io::{AsyncRead, AsyncSeek, AsyncWrite, ReadBuf};

#[derive(From)]
pub enum ByteStream {
    Memory(Cursor<Vec<u8>>),
    File(tokio::fs::File),
}

impl From<Vec<u8>> for ByteStream {
    fn from(v: Vec<u8>) -> Self {
        Self::Memory(Cursor::new(v))
    }
}

impl From<std::fs::File> for ByteStream {
    fn from(f: File) -> Self {
        Self::File(f.into())
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
            ByteStream::File(f) => Pin::new(f).poll_read(cx, buf),
        }
    }
}

impl AsyncSeek for ByteStream {
    fn start_seek(self: Pin<&mut Self>, position: SeekFrom) -> IOResult<()> {
        match self.get_mut() {
            ByteStream::Memory(v) => Pin::new(v).start_seek(position),
            ByteStream::File(f) => Pin::new(f).start_seek(position),
        }
    }

    fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IOResult<u64>> {
        match self.get_mut() {
            ByteStream::Memory(v) => Pin::new(v).poll_complete(cx),
            ByteStream::File(f) => Pin::new(f).poll_complete(cx),
        }
    }
}

impl AsyncWrite for ByteStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        match self.get_mut() {
            ByteStream::Memory(v) => Pin::new(v).poll_write(cx, buf),
            ByteStream::File(f) => Pin::new(f).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        match self.get_mut() {
            ByteStream::Memory(v) => Pin::new(v).poll_flush(cx),
            ByteStream::File(f) => Pin::new(f).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        match self.get_mut() {
            ByteStream::Memory(v) => Pin::new(v).poll_shutdown(cx),
            ByteStream::File(f) => Pin::new(f).poll_flush(cx),
        }
    }
}
