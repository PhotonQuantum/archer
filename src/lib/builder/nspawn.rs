use std::path::{Path, PathBuf};
use std::process::Stdio;

use async_trait::async_trait;
use tokio::io::AsyncWriteExt;

use crate::consts::*;
use crate::error::{CommandError, GpgError};
use crate::parser::PacmanConf;
use crate::parser::GLOBAL_CONFIG;
use crate::utils::map_gpg_code;

use super::{BuildOptions, Builder, Result};

#[derive(Clone)]
pub struct NspawnBuildOptions {
    base: BuildOptions,
    working_dir: PathBuf,
    pacman_conf: Option<PacmanConf>,
    makepkg_conf: Option<PathBuf>,
}

impl NspawnBuildOptions {
    pub fn new(base_option: &BuildOptions, working_dir: impl AsRef<Path>) -> Self {
        Self {
            base: base_option.clone(),
            working_dir: working_dir.as_ref().to_path_buf(),
            pacman_conf: None,
            makepkg_conf: None,
        }
    }
    setter_option_clone!(pacman_conf, PacmanConf);

    pub fn makepkg_conf(mut self, makepkg_conf: impl AsRef<Path>) -> Self {
        self.makepkg_conf = Some(makepkg_conf.as_ref().to_path_buf());
        self
    }
}

#[derive(Clone)]
pub struct NspawnBuilder {
    options: NspawnBuildOptions,
}

impl NspawnBuilder {
    pub fn new(options: &NspawnBuildOptions) -> Self {
        Self {
            options: options.clone(),
        }
    }

    fn pacman_conf(&self) -> &PacmanConf {
        self.options
            .pacman_conf
            .as_ref()
            .map_or(&GLOBAL_CONFIG, |conf| conf)
    }

    fn makepkg_conf(&self) -> PathBuf {
        self.options
            .makepkg_conf
            .as_ref()
            .map_or(PathBuf::from(MAKEPKG_CONF_PATH), Clone::clone)
    }

    fn set_stdout(&self, cmd: &mut tokio::process::Command) {
        if !self.options.base.verbose {
            cmd.stdout(Stdio::null());
            cmd.stderr(Stdio::null());
        }
    }

    pub(crate) async fn test_unshare() -> bool {
        let mut child = if let Ok(child) = tokio::process::Command::new("sudo")
            .args(&["unshare", "--fork", "--pid", "bash", "-c", "exit"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            child
        } else {
            return false;
        };

        child
            .wait()
            .await
            .map(|status| status.success())
            .unwrap_or(false)
    }

    async fn copy_hostconf(&self) -> Result<()> {
        let working_dir = &self.options.working_dir;
        let pacman_conf = self.pacman_conf();
        let makepkg_conf = self.makepkg_conf();

        let src_gpg_dir = PathBuf::from(pacman_conf.option("GPGDir").unwrap());
        let dest_gpg_dir = working_dir.join("etc/pacman.d/gnupg");
        tokio::fs::create_dir_all(&dest_gpg_dir).await?;

        let mut gpg_cmd = if Self::test_unshare().await {
            let mut cmd = tokio::process::Command::new("sudo");
            cmd.args(&["unshare", "--fork", "--pid", "gpg"]);
            cmd
        } else {
            tokio::process::Command::new("gpg")
        };
        gpg_cmd
            .arg("--homedir")
            .arg(&dest_gpg_dir)
            .args(&["--no-permission-warning", "--quiet", "--batch", "--import"])
            .args(&["--import-options", "import-local-sigs"])
            .arg(src_gpg_dir.join("pubring.gpg"));
        self.set_stdout(&mut gpg_cmd);
        let gpg_code = gpg_cmd.spawn()?.wait().await?.code();
        gpg_code
            .map_or(Some(GpgError::Signal), map_gpg_code)
            .map_or(Ok(()), |e| Err(CommandError::Gpg(e)))?;

        let mut key_init_cmd = tokio::process::Command::new("sudo");
        key_init_cmd
            .args(&["pacman-key", "--gpgdir"])
            .arg(&dest_gpg_dir)
            .arg("--init");
        self.set_stdout(&mut key_init_cmd);
        if !key_init_cmd.spawn()?.wait().await?.success() {
            return Err(CommandError::PacmanKey.into());
        }

        let mut key_trust_cmd = tokio::process::Command::new("sudo");
        key_trust_cmd
            .args(&["pacman-key", "--gpgdir"])
            .arg(&dest_gpg_dir)
            .arg("--import-trustdb")
            .arg(&src_gpg_dir);
        self.set_stdout(&mut key_trust_cmd);
        if !key_trust_cmd.spawn()?.wait().await?.success() {
            return Err(CommandError::PacmanKey.into());
        }

        let dest_mirror_list = working_dir.join("etc/pacman.d/mirrorlist");
        let dest_pac_conf = working_dir.join("etc/pacman.conf");
        let dest_makepkg_conf = working_dir.join("etc/makepkg.conf");
        tokio::fs::create_dir_all(dest_mirror_list.parent().unwrap()).await?;
        tokio::fs::File::create(&dest_mirror_list)
            .await?
            .write_all(pacman_conf.mirror_list().as_ref())
            .await?;
        tokio::fs::copy(pacman_conf.path(), &dest_pac_conf).await?;
        tokio::fs::copy(&makepkg_conf, &dest_makepkg_conf).await?;

        // TODO files

        // TODO sed cachedir

        Ok(())
    }
}

#[async_trait]
impl Builder for NspawnBuilder {
    async fn setup(&self) -> Result<()> {
        self.copy_hostconf().await?;

        Ok(())
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
