use std::path::{Path, PathBuf};

use tokio::process::Command;
use async_trait::async_trait;

use crate::builder::Builder;
use crate::error::BuildError;

use super::Result;

pub struct BareBuilder {}

impl BareBuilder {
    async fn system(mut cmd: Command) -> Result<()> {
        let mut child = cmd.spawn()?;

        let status = child.wait().await?;
        if !status.success() {
            Err(BuildError::CommandError)
        } else {
            Ok(())
        }
    }
}

#[async_trait]
impl Builder for BareBuilder {
    async fn setup() -> Result<()> {
        Ok(())
    }

    async fn teardown() -> Result<()> {
        Ok(())
    }

    async fn sync_system() -> Result<()> {
        system(Command::new("sudo")
            .arg("pacman")
            .arg("-Syu"))
    }

    async fn install_local(path: &Path) -> Result<()> {
        system(Command::new("sudo")
            .arg("pacman")
            .arg("-U")
            .arg(path))
    }

    async fn install_remote(package: &str) -> Result<()> {
        system(Command::new("sudo")
            .arg("pacman")
            .arg("-S")
            .arg(package))
    }

    async fn build(path: &Path) -> Result<Vec<PathBuf>> {
        todo!()
    }
}
