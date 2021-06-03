pub use bytestream::*;

use crate::error::StorageError;

mod bytestream;

pub(crate) type Result<T> = std::result::Result<T, StorageError>;
