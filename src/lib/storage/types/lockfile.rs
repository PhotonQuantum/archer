use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::consts::LOCK_FILE_VERSION;
use crate::utils::unix_timestamp;

use super::*;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct LockFile {
    pub version: u32,
    pub timestamp: u128,
    pub packages: HashSet<RemotePackageUnit>,
}

impl Default for LockFile {
    fn default() -> Self {
        Self {
            version: LOCK_FILE_VERSION,
            timestamp: unix_timestamp(),
            packages: HashSet::new(),
        }
    }
}

impl LockFile {
    pub fn new() -> Self {
        Default::default()
    }
}

impl From<&MetaKeyMap> for LockFile {
    fn from(m: &MetaKeyMap) -> Self {
        Self {
            version: LOCK_FILE_VERSION,
            timestamp: unix_timestamp(),
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
