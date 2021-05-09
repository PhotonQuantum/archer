use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Debug;

use crate::types::*;

pub mod aur;
pub mod cached;
pub mod custom;
pub mod empty;
pub mod merged;
pub mod pacman;

#[cfg(test)]
mod tests;

pub trait Repository: Debug + Send + Sync {
    fn find_package(&self, pkg: &Depend) -> Result<Vec<Package>>;
    fn find_packages(&self, pkgs: &[Depend]) -> Result<HashMap<Depend, Vec<Package>>> {
        let mut result = HashMap::new();
        for pkg in pkgs {
            match self.find_package(pkg) {
                Err(e) => return Err(e),
                Ok(v) => {
                    result.insert(pkg.clone(), v);
                }
            }
        }
        Ok(result)
    }
}

fn sort_pkgs_mut(pkgs: &mut Vec<Package>, preferred: &Depend) {
    pkgs.sort_unstable_by(|a, b| {
        if a.name() == preferred.name && b.name() != preferred.name {
            Ordering::Less
        } else if a.name() != preferred.name && b.name() == preferred.name {
            Ordering::Greater
        } else {
            match a
                .partial_cmp(b)
                .unwrap_or_else(|| a.version().cmp(&b.version()))
            {
                Ordering::Less => Ordering::Greater,
                Ordering::Greater => Ordering::Less,
                Ordering::Equal => Ordering::Equal,
            }
        }
    });
}

fn classify_package(
    candidate: Package,
    target_deps: &[Depend],
) -> impl Iterator<Item = Option<(Depend, Package)>> + '_ {
    target_deps.iter().map(move |dep| {
        dep.satisfied_by(&candidate)
            .then_some((dep.clone(), candidate.clone()))
    })
}
