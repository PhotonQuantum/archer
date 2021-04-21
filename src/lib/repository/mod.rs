use crate::types::*;
use std::cmp::Ordering;

pub mod aur;
pub mod pacman;

pub trait Repository {
    fn find_package(&mut self, pkg: &str) -> Result<Vec<Package>>;
}

fn sort_pkgs_mut(pkgs: &mut Vec<Package>, preferred: &str) {
    pkgs.sort_unstable_by(|a, b| {
        if a.name() == preferred && b.name() != preferred {
            Ordering::Less
        } else if a.name() != preferred && b.name() == preferred {
            Ordering::Greater
        } else {
            match a
                .partial_cmp(b)
                .unwrap_or_else(|| a.version().cmp(&b.version()))
            {
                Ordering::Less => Ordering::Greater,
                Ordering::Greater => Ordering::Less,
                ord => ord,
            }
        }
    });
}
