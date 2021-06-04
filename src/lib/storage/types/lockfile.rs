use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::consts::LOCK_FILE_VERSION;

use super::*;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct LockFile {
    version: u32,
    packages: HashSet<RemotePackageUnit>,
}

impl From<&MetaKeyMap> for LockFile {
    fn from(m: &MetaKeyMap) -> Self {
        Self {
            version: LOCK_FILE_VERSION,
            packages: m
                .iter()
                .map(|(meta, key)| RemotePackageUnit {
                    meta: meta.clone(),
                    key: key.clone(),
                })
                .collect(),
        }
    }
}

impl From<&LockFile> for MetaKeyMap {
    fn from(l: &LockFile) -> Self {
        l.packages
            .iter()
            .map(|unit| (unit.meta.clone(), unit.key.clone()))
            .collect()
    }
}
