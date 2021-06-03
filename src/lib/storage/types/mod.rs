use std::collections::HashMap;
use std::path::PathBuf;

pub use bytestream::*;
pub use lockfile::*;
pub use package::*;

use crate::error::StorageError;

mod bytestream;
mod lockfile;
mod package;

pub(crate) type Result<T> = std::result::Result<T, StorageError>;
pub(crate) type MetaKeyMap = HashMap<PackageMeta, PathBuf>;
