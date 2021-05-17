use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use rustympkglib::pkgdata::PkgData;

use crate::error::Result;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CustomPackage {
    pub name: String,
    pub path: PathBuf,
    pub data: PkgData,
}

impl CustomPackage {
    pub fn from_file(name: String, path: PathBuf) -> Result<Self> {
        // TODO error handling
        let mut buffer = String::new();
        File::open(path.clone())?.read_to_string(&mut buffer)?;
        Ok(Self {
            name,
            path,
            data: PkgData::from_source(&*buffer).unwrap(),
        })
    }
}
