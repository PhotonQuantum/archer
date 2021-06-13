use std::path::{Path, PathBuf};

use async_trait::async_trait;

use crate::error::BuildError;

pub use self::bare::*;

mod bare;
#[cfg(test)]
mod tests;

type Result<T> = std::result::Result<T, BuildError>;

macro_rules! setter_copy {
    ($name: ident, $tyty: ty) => {
        pub fn $name(mut self, $name: $tyty) -> Self {
            self.$name = $name;
            self
        }
    };
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct BuildOptions {
    check: bool,
    sign: bool,
    skip_checksum: bool,
    skip_pgp_check: bool,
}

impl BuildOptions {
    pub fn new() -> Self {
        Default::default()
    }
    setter_copy!(check, bool);
    setter_copy!(sign, bool);
    setter_copy!(skip_checksum, bool);
    setter_copy!(skip_pgp_check, bool);
}

#[async_trait]
pub trait Builder {
    async fn setup(&self) -> Result<()>;
    async fn teardown(&self) -> Result<()>;
    async fn sync_system(&self) -> Result<()>;
    async fn install_local(&self, path: &Path) -> Result<()>;
    async fn install_remote(&self, packages: &[&str]) -> Result<()>;
    async fn remove(&self, packages: &[&str]) -> Result<()>;
    async fn build(&self, path: &Path) -> Result<Vec<PathBuf>>;
}
