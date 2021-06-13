use std::path::{Path, PathBuf};

use async_trait::async_trait;

use crate::error::BuildError;

pub use self::bare::*;

mod bare;

type Result<T> = std::result::Result<T, BuildError>;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct BuildOptions {
    check: bool,
    sign: bool
}

#[async_trait]
pub trait Builder {
    async fn setup(&self) -> Result<()>;
    async fn teardown(&self) -> Result<()>;
    async fn sync_system(&self) -> Result<()>;
    async fn install_local(&self, path: &Path) -> Result<()>;
    async fn install_remote(&self, package: &str) -> Result<()>;
    async fn build(&self, path: &Path) -> Result<Vec<PathBuf>>;
}
