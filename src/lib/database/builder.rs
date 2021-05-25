use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use pkginfo::errors::Error as PkgInfoError;
use pkginfo::PkgInfo;
use sha2::{Digest, Sha256};
use tar::Archive as TarArchive;

use crate::error::{Error, Result};
use crate::types::*;

use super::compressor::ArchiveBuilder;
use super::decompressor::ArchiveReader;

pub enum BuildTarget {
    Folder(PathBuf),
    Archive {
        path: PathBuf,
        repo: String,
        desc_builder: ArchiveBuilder,
        files_builder: ArchiveBuilder,
    },
}

impl BuildTarget {
    pub fn new(path: impl AsRef<Path>, archive_repo: Option<&str>) -> Self {
        if let Some(repo) = archive_repo {
            Self::Archive {
                path: path.as_ref().to_path_buf(),
                repo: repo.to_string(),
                desc_builder: Default::default(),
                files_builder: Default::default(),
            }
        } else {
            Self::Folder(path.as_ref().to_path_buf())
        }
    }

    // append package to target
    pub fn append_pkg(&mut self, desc: LocalPackage, files: Vec<String>) -> Result<()> {
        let dir_name =
            PathBuf::from_str(format!("{}-{}", desc.name, desc.version).as_str()).unwrap();
        let desc_content = archlinux_repo_parser::to_string(&desc).unwrap(); // TODO error handling
        let files_content = format!("%FILES%\n{}", files.join("\n"));
        match self {
            BuildTarget::Folder(target) => {
                let pkg_dir = target.join(dir_name);
                fs::create_dir(&pkg_dir)?;
                fs::write(pkg_dir.join("desc"), desc_content)?;
                fs::write(pkg_dir.join("files"), files_content)?;
            }
            BuildTarget::Archive {
                path: _,
                repo: _,
                desc_builder,
                files_builder,
            } => {
                desc_builder.append_data(dir_name.join("desc"), desc_content.as_ref())?;
                files_builder.append_data(dir_name.join("desc"), desc_content.as_ref())?;
                files_builder.append_data(dir_name.join("files"), files_content.as_ref())?;
            }
        }
        Ok(())
    }

    // finalize (only needed by archive)
    pub fn build(self) -> Result<()> {
        if let BuildTarget::Archive {
            path,
            repo,
            desc_builder,
            files_builder,
        } = self
        {
            let desc_path = path.join(format!("{}.db.tar.zst", repo));
            let desc_data = desc_builder.build()?;
            let mut desc_file = File::create(desc_path)?;
            desc_file.write_all(&*desc_data)?;
            desc_file.flush()?;

            let files_path = path.join(format!("{}.files.tar.zst", repo));
            let files_data = files_builder.build()?;
            let mut files_file = File::create(files_path)?;
            files_file.write_all(&*files_data)?;
            files_file.flush()?;
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct DBBuilder {
    pkgs: Vec<PathBuf>,
}

// Build .db & .files folder
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

    pub fn build(&self, mut target: BuildTarget) -> Result<()> {
        for pkg in &self.pkgs {
            Self::build_single(pkg, &mut target)?;
        }
        target.build()?;

        Ok(())
    }

    // parse package files & .PKGINFO
    fn collect_info(mut tar: TarArchive<impl Read>) -> Result<(Vec<String>, Option<PkgInfo>)> {
        let mut files = vec![];
        let mut info = None;
        // iterate archive
        for entry in tar.entries()? {
            let entry = entry?;
            let path = entry.path()?;
            if !path.to_str().unwrap().starts_with('.') {
                // collect package files
                files.push(path.to_string_lossy().to_string());
            }
            if info.is_none() && entry.header().entry_type().is_file() {
                let name = path.file_name().unwrap().to_string_lossy();
                if name == ".PKGINFO" {
                    // parse .PKGINFO
                    info = Some(PkgInfo::parse_file(entry).map_err(|err| match err {
                        PkgInfoError::IoError(e) => Error::IOError(e),
                        PkgInfoError::InvalidPackageFormat => Error::PackageError,
                    })?);
                }
            }
        }
        Ok((files, info))
    }

    // build a single package
    fn build_single(pkg: &Path, target: &mut BuildTarget) -> Result<()> {
        // unarchive package
        let raw = fs::read(pkg)?;
        let archive = ArchiveReader::from_u8(&raw)?;
        let tar = archive.to_tar();

        // parse package files and .PKGINFO
        let (files, info) = Self::collect_info(tar)?;
        let info = info.ok_or(Error::PackageError)?;

        // convert .PKGINFO to desc format
        let desc_builder: LocalPackageBuilder = info.into();
        // TODO PGP
        // add remaining fields
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

        // output files
        target.append_pkg(desc, files)
    }
}
