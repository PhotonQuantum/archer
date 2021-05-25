use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::types::*;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct ArcherDB {
    version: u32,
    required: Vec<ExplicitEntry>,
    packages: Vec<PackageEntry>
}

// packages need to be built in this repository
// for convenience, patches are saved as path, and they will be read into memory as long as
// dependencies are resolved (and hashed when constructing PackageEntry to determine uniqueness)
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct ExplicitEntry {
    dep: Depend,
    patches: Vec<PathBuf>,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct PatchFile {
    filename: String,
    content: String
}

// Used to calculate chksum for PackageEntry, not actually saved into it.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct HashUnit {
    pkgbuild: String,
    patchset: Vec<PatchFile>
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
// packages already saved in a repository OR pending to be built
// used to calculate the diff and be fed into the planner
// Chksum is calculated by HashUnit
pub struct PackageEntry {
    pkg: Package,
    chksum: u64
}