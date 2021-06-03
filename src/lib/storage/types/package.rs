use std::path::PathBuf;

use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::types::*;

lazy_static! {
    static ref RE: Regex = Regex::new(r"\.tar(\..*)?").unwrap();
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PackageMeta {
    pub name: String,
    pub version: Version,
    pub checksum: u64,
}

impl PackageMeta {
    pub fn short_chksum(&self) -> String {
        let mut str_chksum = format!("{:x}", self.checksum);
        str_chksum.truncate(8);
        str_chksum
    }
    pub fn filename(&self) -> String {
        format!("{}-{}-{}", self.name, self.version, self.short_chksum())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct LocalPackageUnit {
    pub meta: PackageMeta,
    pub path: PathBuf,
}

impl LocalPackageUnit {
    fn get_ext(&self) -> &str {
        RE.find(self.path.file_name().unwrap().to_str().unwrap())
            .unwrap()
            .as_str()
    }
    pub fn canonicalize_filename(&self) -> String {
        format!("{}{}", self.meta.filename(), self.get_ext())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct RemotePackageUnit {
    pub meta: PackageMeta,
    pub key: PathBuf,
}
