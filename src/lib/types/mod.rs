pub use alpm::Package as PacmanPackage;
pub use raur::Package as AurPackage;

pub use depend::*;
pub use package::*;
pub use pacman::*;
pub use version::*;

use crate::repository::Repository;
use std::sync::Arc;

mod depend;
mod package;
mod pacman;
mod version;

pub type ArcRepo = Arc<dyn Repository>;
