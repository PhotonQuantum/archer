use std::io::Cursor;
use std::path::PathBuf;

use async_trait::async_trait;
use rusoto_core::RusotoError;
use rusoto_s3::{
    DeleteObjectRequest, GetObjectError, GetObjectRequest, PutObjectRequest, S3Client,
    StreamingBody, S3,
};
use tempfile::tempfile;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::StorageError;
use crate::storage::providers::{get_fullpath, StorageProvider};
use crate::storage::types::ByteStream;

use super::Result;

pub struct S3Storage {
    client: S3Client,
    bucket: String,
    base: PathBuf,
    memory_limit: u64,
}

fn map_get_err(e: RusotoError<GetObjectError>) -> StorageError {
    match e {
        RusotoError::Service(e) => match e {
            GetObjectError::NoSuchKey(k) => StorageError::FileNotExists(PathBuf::from(k)),
            _ => StorageError::S3Error(RusotoError::Service(e).into()),
        },
        _ => StorageError::S3Error(e.into()),
    }
}

#[async_trait]
impl StorageProvider for S3Storage {
    async fn get_file(&self, path: PathBuf) -> Result<ByteStream> {
        let fullpath = get_fullpath(&self.base, &*path)?;

        let req = GetObjectRequest {
            bucket: self.bucket.clone(),
            key: fullpath.to_str().unwrap().to_string(),
            ..Default::default()
        };
        let data = self.client.get_object(req).await.map_err(map_get_err)?;
        let mut src = data.body.unwrap().into_async_read();

        if data
            .content_length
            .map(|l| l > self.memory_limit as i64)
            .unwrap_or(false)
        {
            let sync_dest = tempfile()?;
            let mut dest = File::from_std(sync_dest);

            let length = tokio::io::copy(&mut src, &mut dest).await?;
            dest.flush().await?;

            Ok(ByteStream::File { file: dest, length })
        } else {
            let mut buf = vec![];
            src.read_to_end(&mut buf).await?;

            Ok(ByteStream::Memory(Cursor::new(buf)))
        }
    }

    async fn put_file(&self, path: PathBuf, data: ByteStream) -> Result<()> {
        let fullpath = get_fullpath(&self.base, &*path)?;
        let content_length = data.size();

        let req = PutObjectRequest {
            body: Some(StreamingBody::new(data)),
            bucket: self.bucket.clone(),
            content_length: Some(content_length as i64),
            content_type: None, // TODO read from meta
            key: fullpath.to_str().unwrap().to_string(),
            ..Default::default()
        };

        self.client
            .put_object(req)
            .await
            .map_err(|e| StorageError::S3Error(e.into()))?;

        Ok(())
    }

    async fn delete_file(&self, path: PathBuf) -> Result<()> {
        let fullpath = get_fullpath(&self.base, &*path)?;

        let req = DeleteObjectRequest {
            bucket: self.bucket.clone(),
            key: fullpath.to_str().unwrap().to_string(),
            ..Default::default()
        };

        self.client
            .delete_object(req)
            .await
            .map_err(|e| StorageError::S3Error(e.into()))?;

        Ok(())
    }
}
