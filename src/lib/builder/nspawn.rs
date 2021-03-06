use std::io::Write;
use std::ops::DerefMut;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use fs3::FileExt;
use tempfile::NamedTempFile;

use crate::consts::*;
use crate::error::{BuildError, CommandError, GpgError};
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
    workdir_lock: Arc<Mutex<Option<std::fs::File>>>,
}

impl NspawnBuilder {
    pub fn new(options: &NspawnBuildOptions) -> Self {
        Self {
            options: options.clone(),
            workdir_lock: Arc::new(Mutex::new(None)),
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

    pub(crate) fn lock_workdir(&self) -> Result<()> {
        std::fs::create_dir_all(&self.options.working_dir)?;

        let lock_file = std::fs::File::create(self.options.working_dir.join(".lock"))
            .map_err(|_| BuildError::LockError)?;
        lock_file
            .lock_exclusive()
            .map_err(|_| BuildError::LockError)?;

        *self.workdir_lock.lock().unwrap().deref_mut() = Some(lock_file);
        Ok(())
    }

    pub(crate) fn unlock_workdir(&self) -> Result<()> {
        let mut maybe_lock_file = self.workdir_lock.lock().unwrap();
        if let Some(lock_file) = &mut *maybe_lock_file {
            lock_file.unlock()?;
        }
        if maybe_lock_file.is_some() {
            *maybe_lock_file = None;
        }
        Ok(())
    }

    async fn sudo_cp(
        &self,
        from: impl AsRef<Path> + Send + Sync,
        to: impl AsRef<Path> + Send + Sync,
        recursive: bool,
    ) -> Result<()> {
        let mut cmd = tokio::process::Command::new("sudo");
        cmd.arg("cp");
        if recursive {
            cmd.arg("-R");
        }
        cmd.arg(from.as_ref()).arg(to.as_ref());
        self.set_stdout(&mut cmd);

        if cmd.spawn()?.wait().await?.success() {
            Ok(())
        } else {
            Err(CommandError::Cp.into())
        }
    }

    async fn make_arch_root(&self) -> Result<()> {
        let pacman_conf = self.pacman_conf();
        let makepkg_conf = self.makepkg_conf();
        let cache_dir = pacman_conf.option("CacheDir").unwrap();
        let working_dir = &self.options.working_dir;

        let mut mkarchroot_cmd = tokio::process::Command::new("mkarchroot");
        mkarchroot_cmd
            .arg("-C")
            .arg(pacman_conf.path())
            .arg("-M")
            .arg(makepkg_conf)
            .args(&["-c", cache_dir])
            .arg(working_dir)
            .arg("base-devel");
        self.set_stdout(&mut mkarchroot_cmd);
        if !mkarchroot_cmd.spawn()?.wait().await?.success() {
            return Err(CommandError::MkArchRoot.into());
        }

        Ok(())
    }

    async fn copy_hostconf(&self) -> Result<()> {
        let working_dir = &self.options.working_dir;
        let pacman_conf = self.pacman_conf();
        let makepkg_conf = self.makepkg_conf();

        let src_gpg_dir = PathBuf::from(pacman_conf.option("GPGDir").unwrap());
        let dest_gpg_dir = working_dir.join("etc/pacman.d/gnupg");

        let mut gpg_cmd = tokio::process::Command::new("sudo");
        if Self::test_unshare().await {
            gpg_cmd.args(&["unshare", "--fork", "--pid", "gpg"]);
        } else {
            gpg_cmd.args(&["gpg"]);
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
        {
            let mut temp_mirror_list = NamedTempFile::new()?;
            temp_mirror_list.write_all(pacman_conf.mirror_list().as_ref())?;
            self.sudo_cp(temp_mirror_list.path(), &dest_mirror_list, false)
                .await?;
        }
        self.sudo_cp(pacman_conf.path(), &dest_pac_conf, false)
            .await?;
        self.sudo_cp(&makepkg_conf, &dest_makepkg_conf, false)
            .await?;

        // TODO files

        // TODO sed cachedir

        Ok(())
    }
}

#[async_trait]
impl Builder for NspawnBuilder {
    async fn setup(&self) -> Result<()> {
        self.make_arch_root().await?;

        self.copy_hostconf().await?;

        // TODO mkchrootpkg

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
