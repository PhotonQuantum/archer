use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use alpm::Alpm;
use itertools::Itertools;

use crate::alpm::GLOBAL_ALPM;
use crate::repository::{classify_package, sort_pkgs_mut, Repository};
use crate::types::*;

#[derive(Clone, Debug)]
pub struct PacmanRemote {
    alpm: Arc<Mutex<Alpm>>,
}

#[derive(Clone, Debug)]
pub struct PacmanLocal {
    alpm: Arc<Mutex<Alpm>>,
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
        }
    }
}

impl Repository for PacmanRemote {
    fn find_package(&self, pkg: &str) -> Result<Vec<Package>> {
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
        sort_pkgs_mut(&mut result, pkg);
        Ok(result)
    }
    fn find_packages(&self, pkgs: &[&str]) -> Result<HashMap<String, Vec<Package>>> {
        // TODO error handling
        let mut result = self
            .alpm // acquired remote dbs
            .lock()
            .unwrap()
            .syncdbs()
            .iter()
            .map(|db| db.search(pkgs.iter().map(|s| s.to_string())).unwrap()) // search for package in all dbs
            .flatten()
            .map(Package::from) // convert to owned
            .map(|p| classify_package(p, pkgs)) // classify packages by requested package name
            .flatten() // collect into map
            .flatten() // remove None
            .into_group_map();

        for (pkgname, pkgs) in &mut result {
            sort_pkgs_mut(pkgs, pkgname);
        }

        Ok(result)
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
        }
    }
}

// NOTE this repository only returns exact match
impl Repository for PacmanLocal {
    fn find_package(&self, pkg: &str) -> Result<Vec<Package>> {
        Ok(self
            .alpm // acquire local db
            .lock()
            .unwrap()
            .localdb()
            .pkgs() // find exact match
            .find_satisfier(pkg)
            .map(|p| vec![Package::from(p)]) // convert to owned
            .unwrap_or_default())
    }

    fn find_packages(&self, pkgs: &[&str]) -> Result<HashMap<String, Vec<Package>>> {
        Ok(self
            .alpm // acquire local db
            .lock()
            .unwrap()
            .localdb()
            .search(pkgs.iter().map(|s| s.to_string()))? // find exact match
            .iter()
            .map(Package::from) // convert to owned
            .map(|p| classify_package(p, pkgs)) // classify packages by requested package name
            .flatten() // collect into map
            .flatten() // remove None
            .into_group_map())
    }
}
