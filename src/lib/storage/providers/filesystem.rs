use std::io::Cursor;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use tempfile::tempfile;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::StorageError;
use crate::storage::providers::StorageProvider;
use crate::storage::types::*;

use super::Result;

pub struct FSStorage {
    base: PathBuf,
    memory_limit: u64,
}

impl FSStorage {
    pub fn new(base: impl AsRef<Path>) -> Self {
        Self {
            base: base.as_ref().to_path_buf(),
            memory_limit: 104857600, // 100 MB
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
    async fn get_file(&self, path: PathBuf) -> Result<ByteStream> {
        let fullpath = self.base.join(&path);
        if !fullpath.starts_with(&self.base) {
            return Err(StorageError::InvalidPath(path));
        }

        let mut src = File::open(&fullpath).await?;
        if src.metadata().await?.len() > self.memory_limit {
            let sync_dest = tempfile()?;
            let mut dest = File::from_std(sync_dest);

            tokio::io::copy(&mut src, &mut dest).await?;
            dest.flush().await?;

            Ok(ByteStream::File(dest))
        } else {
            let mut buf = vec![];
            src.read_to_end(&mut buf).await?;

            Ok(ByteStream::Memory(Cursor::new(buf)))
        }
    }

    async fn put_file(&self, path: PathBuf, data: &mut ByteStream) -> Result<()> {
        let fullpath = self.base.join(&path);
        if !fullpath.starts_with(&self.base) {
            return Err(StorageError::InvalidPath(path));
        }
        if path_exists(&fullpath).await {
            return Err(StorageError::FileExists(path));
        }

        let mut dest = File::create(&fullpath).await?;
        tokio::io::copy(data, &mut dest).await?;

        Ok(())
    }

    async fn delete_file(&self, path: PathBuf) -> Result<()> {
        let fullpath = self.base.join(&path);
        if !fullpath.starts_with(&self.base) {
            return Err(StorageError::InvalidPath(path));
        }
        if !file_exists(&fullpath).await {
            return Err(StorageError::FileNotExists(path));
        }

        tokio::fs::remove_file(fullpath).await?;

        Ok(())
    }

    async fn rename_file(&self, old_path: PathBuf, new_path: PathBuf) -> Result<()> {
        let old_fullpath = self.base.join(&old_path);
        let new_fullpath = self.base.join(&new_path);
        if !old_fullpath.starts_with(&self.base) {
            return Err(StorageError::InvalidPath(old_fullpath));
        }
        if !new_fullpath.starts_with(&self.base) {
            return Err(StorageError::InvalidPath(new_fullpath));
        }
        if !file_exists(&old_fullpath).await {
            return Err(StorageError::FileNotExists(old_fullpath));
        }
        if file_exists(&new_fullpath).await {
            return Err(StorageError::FileNotExists(new_fullpath));
        }

        tokio::fs::rename(old_fullpath, new_fullpath).await?;
        todo!()
    }
}
