use std::io::Cursor;
use std::path::Path;

use tar::{Builder, Header};

use crate::error::Result;

pub struct ArchiveBuilder {
    builder: Builder<Vec<u8>>,
}

impl Default for ArchiveBuilder {
    fn default() -> Self {
        let builder = Builder::new(vec![]);
        Self { builder }
    }
}

impl ArchiveBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn append_data(&mut self, path: impl AsRef<Path>, data: &[u8]) -> Result<()> {
        let mut header = Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        self.builder.append_data(&mut header, path, data)?;
        Ok(())
    }

    pub fn build(mut self) -> Result<Vec<u8>> {
        self.builder.finish()?;
        let tar = self.builder.into_inner()?;
        Ok(zstd::encode_all(Cursor::new(tar), 0)?)
    }
}
