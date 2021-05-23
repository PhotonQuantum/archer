use std::fs::File;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use tar::Archive as TarArchive;

use crate::error::{Error, Result};

pub struct Archive {
    path: PathBuf,
    data: Vec<u8>,
}

impl Archive {
    pub fn from_file(path: &Path) -> Result<Self> {
        let mime = tree_magic::from_filepath(path);
        let mut file = File::open(path)?;
        let mut buf: Vec<u8> = Vec::new();
        match mime.as_str() {
            "application/zstd" => {
                zstd::stream::copy_decode(file, &mut buf)?;
            }
            "application/gzip" => {
                let mut decoder = flate2::read::GzDecoder::new(file);
                decoder.read_to_end(&mut buf)?;
            }
            "application/x-xz" => {
                let mut decoder = xz2::read::XzDecoder::new(file);
                decoder.read_to_end(&mut buf)?;
            }
            "application/x-tar" => {
                file.read_to_end(&mut buf)?;
            }
            _ => return Err(Error::ArchiveError),
        }
        Ok(Self {
            path: path.to_path_buf(),
            data: buf,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn to_tar(&self) -> TarArchive<Cursor<&Vec<u8>>> {
        TarArchive::new(Cursor::new(&self.data))
    }

    pub fn into_tar(self) -> TarArchive<Cursor<Vec<u8>>> {
        TarArchive::new(Cursor::new(self.data))
    }

    pub fn inner(&self) -> &[u8] {
        &self.data
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.data
    }
}
