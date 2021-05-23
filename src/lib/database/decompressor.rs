use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;

use tar::Archive as TarArchive;

use crate::error::{Error, Result};

pub struct Archive {
    data: Vec<u8>,
}

impl Archive {
    #[allow(clippy::unused_io_amount)]
    pub fn from_reader(mut reader: impl Read) -> Result<Self> {
        let mut head = [0; 2048];
        let head_bytes = reader.read(&mut head)?;
        let mime = tree_magic::from_u8(&head);

        let mut reader = if head_bytes == 2048 {
            Box::new(Cursor::new(head).chain(reader)) as Box<dyn Read>
        } else {
            Box::new(&head[..head_bytes]) as Box<dyn Read>
        };
        let mut data: Vec<u8> = Vec::new();
        match mime.as_str() {
            "application/zstd" => {
                zstd::stream::copy_decode(reader, &mut data)?;
            }
            "application/gzip" => {
                let mut decoder = flate2::read::GzDecoder::new(reader);
                decoder.read_to_end(&mut data)?;
            }
            "application/x-xz" => {
                let mut decoder = xz2::read::XzDecoder::new(reader);
                decoder.read_to_end(&mut data)?;
            }
            "application/x-tar" => {
                reader.read_to_end(&mut data)?;
            }
            _ => return Err(Error::ArchiveError),
        }
        Ok(Self { data })
    }
    pub fn from_u8(file: &[u8]) -> Result<Self> {
        Self::from_reader(file)
    }
    pub fn from_filepath(path: &Path) -> Result<Self> {
        Self::from_reader(File::open(path)?)
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
