use std::path::{Path, PathBuf};

use async_trait::async_trait;

use crate::error::StorageError;

use super::types::*;

mod filesystem;
mod s3;

pub use filesystem::*;
pub use s3::*;

type Result<T> = std::result::Result<T, StorageError>;

#[async_trait]
pub trait StorageProvider {
    async fn get_file(&self, path: &Path) -> Result<ByteStream>;
    // async fn get_file_meta();
    async fn put_file(&self, path: &Path, data: ByteStream) -> Result<()>;
    // fn set_file_meta();
    async fn delete_file(&self, path: &Path) -> Result<()>;
    // async fn list_files(prefix: String) -> ;
}

fn get_fullpath(base: &Path, path: &Path) -> Result<PathBuf> {
    let fullpath = base.join(path);
    if !fullpath.starts_with(base) {
        return Err(StorageError::InvalidPath(path.to_path_buf()));
    }
    Ok(fullpath)
}
