use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use pkginfo::errors::Error as PkgInfoError;
use pkginfo::PkgInfo;
use sha2::{Digest, Sha256};
use tar::Archive as TarArchive;

use crate::error::{Error, Result};
use crate::types::*;

use super::decompressor::Archive;

#[derive(Debug, Default, Clone)]
pub struct DBBuilder {
    pkgs: Vec<PathBuf>,
}

impl DBBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_file(mut self, file: PathBuf) -> Self {
        self.add_file_mut(file);
        self
    }

    pub fn add_file_mut(&mut self, file: PathBuf) {
        self.pkgs.push(file);
    }

    pub fn build<T: AsRef<Path>>(&self, path: T) -> Result<()> {
        let path = path.as_ref();
        for pkg in &self.pkgs {
            Self::build_single(pkg, path)?;
        }

        Ok(())
    }

    fn collect_info(mut tar: TarArchive<impl Read>) -> Result<(Vec<String>, Option<PkgInfo>)> {
        let mut files = vec![];
        let mut info = None;
        for entry in tar.entries()? {
            let entry = entry?;
            let path = entry.path()?;
            if !path.to_str().unwrap().starts_with('.') {
                files.push(path.to_string_lossy().to_string());
            }
            if info.is_none() && entry.header().entry_type().is_file() {
                let name = path.file_name().unwrap().to_string_lossy();
                if name == ".PKGINFO" {
                    info = Some(PkgInfo::parse_file(entry).map_err(|err| match err {
                        PkgInfoError::IoError(e) => Error::IOError(e),
                        PkgInfoError::InvalidPackageFormat => Error::PackageError,
                    })?);
                }
            }
        }
        Ok((files, info))
    }

    fn build_single(pkg: &Path, target: &Path) -> Result<()> {
        let raw = fs::read(pkg)?;
        let archive = Archive::from_u8(&raw)?;
        let tar = archive.to_tar();

        let (files, info) = Self::collect_info(tar)?;
        let info = info.ok_or(Error::PackageError)?;

        let desc_builder: LocalPackageBuilder = info.into();
        // TODO PGP
        let desc: LocalPackage = desc_builder
            .file_name(pkg.file_name().unwrap().to_string_lossy().to_string())
            .compressed_size(fs::metadata(pkg)?.len())
            .md5_sum(format!("{:?}", md5::compute(&raw)))
            .sha256_sum({
                let mut hasher = Sha256::new();
                hasher.update(&raw);
                format!("{:x}", hasher.finalize())
            })
            .build()
            .unwrap();

        let pkg_dir = target.join(format!("{}-{}", desc.name, desc.version));
        fs::create_dir(&pkg_dir)?;
        fs::write(
            pkg_dir.join("desc"),
            archlinux_repo_parser::to_string(&desc).unwrap(),
        )?;
        fs::write(
            pkg_dir.join("files"),
            format!("%FILES%\n{}", files.join("\n")),
        )?;

        Ok(())
    }
}
