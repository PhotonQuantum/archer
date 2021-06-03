use std::io::{Cursor, SeekFrom};
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use rusoto_core::credential::StaticProvider;
use rusoto_core::{Client, Region, RusotoError};
use rusoto_s3::{
    DeleteObjectRequest, GetObjectError, GetObjectRequest, PutObjectRequest, S3Client,
    StreamingBody, S3,
};
use tempfile::NamedTempFile;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

use crate::consts::STORAGE_MEMORY_LIMIT;
use crate::error::{S3Error, StorageError};
use crate::storage::providers::{get_fullpath, StorageProvider};
use crate::storage::types::ByteStream;

use super::Result;

pub struct S3Storage {
    client: S3Client,
    bucket: String,
    base: PathBuf,
    memory_limit: u64,
}

#[derive(Clone, Eq, PartialEq, Default, Hash)]
pub struct S3StorageBuilder {
    name: Option<String>,
    endpoint: Option<String>,
    credential: Option<(String, String)>,
    bucket: Option<String>,
    base: Option<PathBuf>,
    memory_limit: Option<u64>,
}

impl S3StorageBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_name(self, name: impl ToString) -> Self {
        Self {
            name: Some(name.to_string()),
            ..self
        }
    }

    pub fn with_endpoint(self, endpoint: impl ToString) -> Self {
        Self {
            endpoint: Some(endpoint.to_string()),
            ..self
        }
    }

    pub fn with_credential(self, key: impl ToString, secret: impl ToString) -> Self {
        Self {
            credential: Some((key.to_string(), secret.to_string())),
            ..self
        }
    }

    pub fn with_bucket(self, bucket: impl ToString) -> Self {
        Self {
            bucket: Some(bucket.to_string()),
            ..self
        }
    }

    pub fn with_base(self, base: impl AsRef<Path>) -> Self {
        Self {
            base: Some(base.as_ref().to_path_buf()),
            ..self
        }
    }

    pub fn with_memory_limit(self, memory_limit: u64) -> Self {
        Self {
            memory_limit: Some(memory_limit),
            ..self
        }
    }

    pub fn build_with_client(self, client: S3Client) -> Result<S3Storage> {
        Ok(S3Storage {
            client,
            bucket: self
                .bucket
                .ok_or_else(|| S3Error::BuilderError(String::from("missing bucket field")))?,
            base: self.base.unwrap_or_default(),
            memory_limit: self.memory_limit.unwrap_or(STORAGE_MEMORY_LIMIT),
        })
    }

    pub fn build(self) -> Result<S3Storage> {
        let (key, secret) = self
            .credential
            .ok_or_else(|| S3Error::BuilderError(String::from("missing credential field")))?;
        let name = self
            .name
            .ok_or_else(|| S3Error::BuilderError(String::from("missing name field")))?;
        let endpoint = self
            .endpoint
            .ok_or_else(|| S3Error::BuilderError(String::from("missing endpoint field")))?;
        let bucket = self
            .bucket
            .ok_or_else(|| S3Error::BuilderError(String::from("missing bucket field")))?;

        let credential = StaticProvider::new_minimal(key, secret);
        let http_client = rusoto_core::HttpClient::new().unwrap();
        let common_client = Client::new_with(credential, http_client);

        let region = Region::Custom { name, endpoint };
        let s3_client = S3Client::new_with_client(common_client, region);

        Ok(S3Storage {
            client: s3_client,
            bucket,
            base: self.base.unwrap_or_default(),
            memory_limit: self.memory_limit.unwrap_or(STORAGE_MEMORY_LIMIT),
        })
    }
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

async fn guess_mime(stream: &mut ByteStream) -> Option<&str> {
    let mut buf = [0; 512];
    let bytes = stream.read(&mut buf).await.unwrap();
    stream
        .seek(SeekFrom::Current(-(bytes as i64)))
        .await
        .unwrap();
    infer::get(&buf).map(|mime| mime.mime_type())
}

#[async_trait]
impl StorageProvider for S3Storage {
    async fn get_file(&self, path: &Path) -> Result<ByteStream> {
        let fullpath = get_fullpath(&self.base, path)?;

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
            let sync_dest = NamedTempFile::new()?;
            let mut dest = File::from_std(sync_dest.reopen()?);

            tokio::io::copy(&mut src, &mut dest).await?;
            dest.flush().await?;

            Ok(ByteStream::from(sync_dest))
        } else {
            let mut buf = vec![];
            src.read_to_end(&mut buf).await?;

            Ok(ByteStream::Memory(Cursor::new(buf)))
        }
    }

    async fn put_file(&self, path: &Path, mut data: ByteStream) -> Result<()> {
        let fullpath = get_fullpath(&self.base, path)?;
        let content_length = data.size();
        let content_type = guess_mime(&mut data).await.map(ToString::to_string);

        let req = PutObjectRequest {
            body: Some(StreamingBody::new(data)),
            bucket: self.bucket.clone(),
            content_length: Some(content_length as i64),
            content_type,
            key: fullpath.to_str().unwrap().to_string(),
            ..Default::default()
        };

        self.client
            .put_object(req)
            .await
            .map_err(|e| StorageError::S3Error(e.into()))?;

        Ok(())
    }

    async fn delete_file(&self, path: &Path) -> Result<()> {
        let fullpath = get_fullpath(&self.base, path)?;

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
