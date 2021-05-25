use std::sync::Arc;

pub use alpm::Package as PacmanPackage;
pub use raur::Package as AurPackage;

pub use custom_package::*;
pub use depend::*;
pub use pacman::*;
pub use remote_package::*;
pub use version::*;

use crate::repository::Repository;

mod custom_package;
mod depend;
mod pacman;
mod remote_package;
mod version;

pub type ArcRepo = Arc<dyn Repository>;
pub type ArcPackage = Arc<Package>;
