pub mod aur;
pub mod pacman;
use crate::types::*;
use async_trait::async_trait;
use std::borrow::Borrow;
use std::fmt::Debug;

pub trait Repository {
    fn find_package(&self, pkg: &str) -> Result<Vec<Package>>;
}