use std::convert::TryFrom;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use tempfile::NamedTempFile;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

use crate::consts::STORAGE_MEMORY_LIMIT;
use crate::error::StorageError;
use crate::storage::types::*;

use super::Result;
use super::{get_fullpath, StorageProvider};

pub struct FSStorage {
    base: PathBuf,
    memory_limit: u64,
}

impl FSStorage {
    pub fn new(base: impl AsRef<Path>) -> Self {
        Self {
            base: base.as_ref().to_path_buf(),
            memory_limit: STORAGE_MEMORY_LIMIT,
        }
    }

    pub fn new_with_limit(base: impl AsRef<Path>, memory_limit: u64) -> Self {
        Self {
            base: base.as_ref().to_path_buf(),
            memory_limit,
        }
    }
}

async fn path_exists(path: &Path) -> bool {
    tokio::fs::metadata(path).await.is_ok()
}

async fn file_exists(path: &Path) -> bool {
    tokio::fs::metadata(path)
        .await
        .map(|m| m.is_file())
        .unwrap_or(false)
}

#[async_trait]
impl StorageProvider for FSStorage {
    async fn get_file(&self, path: &Path) -> Result<ByteStream> {
        let fullpath = get_fullpath(&*self.base, path)?;
        if !file_exists(&fullpath).await {
            return Err(StorageError::FileNotExists(path.to_path_buf()));
        }

        let mut src = File::open(&fullpath).await?;
        if src.metadata().await?.len() > self.memory_limit {
            let sync_dest = NamedTempFile::new()?;
            let mut dest = File::from_std(sync_dest.reopen()?);

            tokio::io::copy(&mut src, &mut dest).await?;
            dest.sync_all().await?;

            Ok(ByteStream::try_from(sync_dest)?)
        } else {
            let mut buf = vec![];
            src.read_to_end(&mut buf).await?;

            Ok(ByteStream::Memory(Cursor::new(buf)))
        }
    }

    async fn put_file(&self, path: &Path, mut data: ByteStream) -> Result<()> {
        let fullpath = get_fullpath(&*self.base, path)?;
        if path_exists(&fullpath).await {
            return Err(StorageError::FileExists(path.to_path_buf()));
        }

        let mut dest = File::create(&fullpath).await?;
        tokio::io::copy(&mut data, &mut dest).await?;

        Ok(())
    }

    async fn delete_file(&self, path: &Path) -> Result<()> {
        let fullpath = get_fullpath(&*self.base, path)?;
        if !file_exists(&fullpath).await {
            return Err(StorageError::FileNotExists(path.to_path_buf()));
        }

        tokio::fs::remove_file(fullpath).await?;

        Ok(())
    }
}
