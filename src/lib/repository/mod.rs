use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Debug;

use crate::types::*;

pub mod aur;
pub mod cached;
pub mod pacman;
mod merged;

pub trait Repository: Debug + Send + Sync {
    fn find_package(&self, pkg: &str) -> Result<Vec<Package>> {
        Ok(self
            .find_packages([pkg].as_ref())?
            .remove(pkg)
            .unwrap())
    }
    fn find_packages(&self, pkgs: &[&str]) -> Result<HashMap<String, Vec<Package>>>;
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

fn classify_package<'a>(
    pkg: Package,
    preferred_pkgs: &'a [&str],
) -> impl Iterator<Item = Option<(String, Package)>> + 'a {
    preferred_pkgs.iter().map(move |pkgname| {
        (pkg.name() == *pkgname
            || pkg
                .provides()
                .into_iter()
                .any(|provide| provide.name == *pkgname))
        .then_some((pkgname.to_string(), pkg.clone()))
    })
}
