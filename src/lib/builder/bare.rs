use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use tokio::process::Command;
use tokio::sync::Mutex;

use crate::builder::{BuildOptions, Builder};
use crate::error::{BuildError, CommandError, MakepkgError};
use crate::utils::map_makepkg_code;

use super::{IOResult, Result};

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct BareBuildOptions {
    base: BuildOptions,
    build_as: Option<String>,
}

impl BareBuildOptions {
    pub fn new(base_option: &BuildOptions) -> Self {
        Self {
            base: base_option.clone(),
            build_as: None,
        }
    }
    pub fn build_as(mut self, user: &str) -> Self {
        self.build_as = Some(user.to_string());
        self
    }
}

#[derive(Debug, Default)]
pub struct BareBuilder {
    pacman_lock: Mutex<()>,
    options: BareBuildOptions,
}

impl BareBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_with_options(options: &BareBuildOptions) -> Self {
        Self {
            pacman_lock: Default::default(),
            options: options.clone(),
        }
    }

    async fn pacman<S: AsRef<OsStr>, T: IntoIterator<Item = S> + Send>(
        &self,
        args: T,
    ) -> Result<()> {
        let _lock = self.pacman_lock.lock().await;
        let mut cmd = Command::new("sudo");
        cmd.arg("pacman");
        cmd.arg("--noconfirm");
        for arg in args {
            cmd.arg(arg);
        }
        let mut child = cmd.spawn()?;

        let status = child.wait().await?;
        if status.success() {
            Ok(())
        } else {
            Err(BuildError::CommandError(CommandError::Pacman))
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
        self.pacman(&[OsStr::new("-U"), path.as_os_str(), OsStr::new("--needed")])
            .await
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
        let output_dir = path.join("output");
        let mut cmd = if let Some(user) = &self.options.build_as {
            let mut cmd = Command::new("sudo");
            cmd.arg("-u");
            cmd.arg(user);
            cmd.arg(format!("PKGDEST={}", output_dir.to_str().unwrap()));
            cmd.arg("makepkg");
            cmd
        } else {
            let mut cmd = Command::new("makepkg");
            cmd.env("PKGDEST", &output_dir);
            cmd
        };

        if !output_dir.exists() {
            tokio::fs::create_dir(&output_dir).await?;
            if let Some(user) = &self.options.build_as {
                let status = Command::new("sudo")
                    .arg("chown")
                    .arg("-R")
                    .arg(format!("{}:{}", user, user))
                    .arg(&path)
                    .spawn()?
                    .wait()
                    .await?;
                if !status.success() {
                    return Err(BuildError::CommandError(CommandError::Chown));
                }
            }
        }

        cmd.current_dir(path).env("PKGDEST", &output_dir);

        if self.options.base.check {
            cmd.arg("--check");
        }
        if self.options.base.sign {
            cmd.arg("--sign");
        }
        if self.options.base.skip_checksum {
            cmd.arg("--skipchecksums");
        }
        if self.options.base.skip_pgp_check {
            cmd.arg("--skippgpcheck");
        }

        let mut child = cmd.spawn()?;
        let status = child.wait().await?;

        status
            .code()
            .map_or(Some(MakepkgError::Signal), map_makepkg_code)
            .map_or(Ok(()), |e| Err(CommandError::Makepkg(e)))?;

        if self.options.build_as.is_some() {
            let status = Command::new("sudo")
                .arg("chown")
                .arg("-R")
                .arg(format!(
                    "{}:{}",
                    users::get_current_uid().to_string(),
                    users::get_current_gid().to_string()
                ))
                .arg(&output_dir)
                .spawn()?
                .wait()
                .await?;
            if !status.success() {
                return Err(BuildError::CommandError(CommandError::Chown));
            }
        }

        Ok(std::fs::read_dir(&output_dir)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<IOResult<Vec<_>>>()?)
    }
}
