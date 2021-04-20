use crate::types::*;

pub mod aur;
pub mod pacman;

pub trait Repository {
    fn find_package(&mut self, pkg: &str) -> Result<Vec<Package>>;
}
