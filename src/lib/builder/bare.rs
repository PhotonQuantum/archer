use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use tokio::process::Command;

use crate::builder::{BuildOptions, Builder};
use crate::error::{BuildError, CommandError, MakepkgError};
use crate::utils::map_makepkg_code;

use super::Result;
use tokio::sync::Mutex;

type IOResult<T> = std::result::Result<T, std::io::Error>;

#[derive(Debug, Default)]
pub struct BareBuilder {
    pacman_lock: Mutex<()>,
    options: BuildOptions,
}

impl BareBuilder {
    fn new() -> Self {
        Default::default()
    }

    fn new_with_options(options: &BuildOptions) -> Self {
        Self {
            pacman_lock: Default::default(),
            options: options.clone(),
        }
    }

    async fn pacman<S: AsRef<OsStr>>(&self, args: &[S]) -> Result<()> {
        let _lock = self.pacman_lock.lock().await;
        let mut cmd = Command::new("sudo");
        cmd.arg("pacman");
        for arg in args {
            cmd.arg(arg);
        }
        let mut child = cmd.spawn()?;

        let status = child.wait().await?;
        if !status.success() {
            Err(BuildError::CommandError(CommandError::Pacman))
        } else {
            Ok(())
        }
    }
}

#[async_trait]
impl Builder for BareBuilder {
    async fn setup(&self) -> Result<()> {
        Ok(())
    }

    async fn teardown(&self) -> Result<()> {
        Ok(())
    }

    async fn sync_system(&self) -> Result<()> {
        self.pacman(&["-Syu"]).await
    }

    async fn install_local(&self, path: &Path) -> Result<()> {
        self.pacman(&[OsStr::new("-U"), path.as_os_str(), OsStr::new("--needed")]).await
    }

    async fn install_remote(&self, packages: &[&str]) -> Result<()> {
        let mut args = vec!["-S"];
        args.extend(packages);
        args.push("--needed");
        self.pacman(&args).await
    }

    async fn remove(&self, packages: &[&str]) -> Result<()> {
        let mut args = vec!["-R"];
        args.extend(packages);
        self.pacman(&args).await
    }

    async fn build(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let mut cmd = Command::new("makepkg");
        cmd.current_dir(path).env("PKGDEST", path.join("output"));

        if self.options.check {
            cmd.arg("--check");
        }
        if self.options.sign {
            cmd.arg("--sign");
        }
        if self.options.skip_checksum {
            cmd.arg("--skipchecksums");
        }
        if self.options.skip_pgp_check {
            cmd.arg("--skippgpcheck");
        }

        let mut child = cmd.spawn()?;
        let status = child.wait().await?;

        status
            .code()
            .map_or(Some(MakepkgError::Signal), map_makepkg_code)
            .map(|e| Err(CommandError::Makepkg(e)))
            .unwrap_or(Ok(()))?;

        Ok(std::fs::read_dir(path.join("output"))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<IOResult<Vec<_>>>()?)
    }
}
