pub mod aur;
pub mod pacman;
use crate::types::*;

pub trait Repository {
    fn find_package(&mut self, pkg: &str) -> Result<Vec<Package>>;
}
