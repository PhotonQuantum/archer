use std::path::{Path, PathBuf};

use async_trait::async_trait;

use crate::builder::{BuildOptions, Builder};

use super::{IOResult, Result};

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct NspawnBuildOptions {
    base: BuildOptions,
}

impl NspawnBuildOptions {
    pub fn new(base_option: &BuildOptions) -> Self {
        Self {
            base: base_option.clone(),
        }
    }
}

#[derive(Debug, Default)]
pub struct NspawnBuilder {
    options: NspawnBuildOptions,
}

impl NspawnBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_with_options(options: &NspawnBuildOptions) -> Self {
        Self {
            options: options.clone(),
        }
    }
}

#[async_trait]
impl Builder for NspawnBuilder {
    async fn setup(&self) -> Result<()> {
        todo!()
    }

    async fn teardown(&self) -> Result<()> {
        todo!()
    }

    async fn sync_system(&self) -> Result<()> {
        todo!()
    }

    async fn install_local(&self, path: &Path) -> Result<()> {
        todo!()
    }

    async fn install_remote(&self, packages: &[&str]) -> Result<()> {
        todo!()
    }

    async fn remove(&self, packages: &[&str]) -> Result<()> {
        todo!()
    }

    async fn build(&self, path: &Path) -> Result<Vec<PathBuf>> {
        todo!()
    }
}
