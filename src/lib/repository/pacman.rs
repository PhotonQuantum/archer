use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use alpm::Alpm;

use crate::alpm::GLOBAL_ALPM;
use crate::repository::Repository;
use crate::types::*;

#[derive(Clone)]
pub struct PacmanRemote {
    alpm: Arc<Mutex<Alpm>>,
    cache: HashMap<String, Vec<Package>>,
}

#[derive(Clone)]
pub struct PacmanLocal {
    alpm: Arc<Mutex<Alpm>>,
    cache: HashMap<String, Vec<Package>>,
}

impl PacmanRemote {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for PacmanRemote {
    fn default() -> Self {
        Self {
            alpm: GLOBAL_ALPM.clone(),
            cache: Default::default(),
        }
    }
}

impl Repository for PacmanRemote {
    fn find_package(&mut self, pkg: &str) -> Result<Vec<Package>> {
        if let Some(pkg) = self.cache.get(pkg) {
            Ok(pkg.to_vec())
        } else {
            // let result: Vec<Package> = self
            //     .alpm
            //     .lock()
            //     .unwrap()
            //     .syncdbs()
            //     .find_satisfier(pkg)
            //     .map(|p| vec![p.into()])
            //     .unwrap_or_else(Vec::new);
            // TODO error handling
            let mut result: Vec<Package> = self
                .alpm
                .lock()
                .unwrap()
                .syncdbs()
                .iter()
                .map(|db| db.search([pkg.to_string()].iter()).unwrap())
                .flatten()
                .map(Package::from)
                .filter(|p| {
                    p.name() == pkg || p.provides().into_iter().any(|provide| provide.name == pkg)
                })
                .collect();
            result.sort_unstable_by(|a, b| {
                if a.name() == pkg && b.name() != pkg {
                    Ordering::Less
                } else if a.name() != pkg && b.name() == pkg {
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
            self.cache.insert(pkg.to_string(), result.clone());
            Ok(result)
        }
    }
}

impl PacmanLocal {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for PacmanLocal {
    fn default() -> Self {
        Self {
            alpm: GLOBAL_ALPM.clone(),
            cache: Default::default(),
        }
    }
}

impl Repository for PacmanLocal {
    fn find_package(&mut self, pkg: &str) -> Result<Vec<Package>> {
        if let Some(pkg) = self.cache.get(pkg) {
            Ok(pkg.to_vec())
        } else {
            Ok(self
                .alpm
                .lock()
                .unwrap()
                .localdb()
                .pkgs()
                .find_satisfier(pkg)
                .map(|p| vec![p.into()])
                .unwrap_or_else(Vec::new))
        }
    }
}
