use std::path::PathBuf;

use async_trait::async_trait;

use crate::error::StorageError;

use super::types::*;

mod filesystem;

type Result<T> = std::result::Result<T, StorageError>;

#[async_trait]
pub trait StorageProvider {
    async fn get_file(&self, path: PathBuf) -> Result<ByteStream>;
    // async fn get_file_meta();
    async fn put_file(&self, path: PathBuf, data: &mut ByteStream) -> Result<()>;
    // fn set_file_meta();
    async fn delete_file(&self, path: PathBuf) -> Result<()>;
    // async fn list_files(prefix: String) -> ;
    async fn rename_file(&self, old_path: PathBuf, new_path: PathBuf) -> Result<()>;
}
