use std::sync::{Arc, Mutex};

use alpm::Alpm;
use itertools::Itertools;

use crate::alpm::GLOBAL_ALPM;
use crate::repository::{sort_pkgs_mut, Repository};
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
    fn find_package(&self, pkg: &Depend) -> Result<Vec<Package>> {
        let mut result = self
            .alpm
            .lock()
            .unwrap()
            .syncdbs()
            .iter()
            .map(|db| db.search([pkg.name.clone()].iter()))
            .try_collect::<_, Vec<_>, _>()?
            .into_iter()
            .flat_map(|pkgs| {
                pkgs.into_iter()
                    .map(Package::from)
                    .filter(|candidate| pkg.satisfied_by(candidate))
            })
            .collect();
        sort_pkgs_mut(&mut result, pkg);
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
    fn find_package(&self, pkg: &Depend) -> Result<Vec<Package>> {
        Ok(self
            .alpm // acquire local db
            .lock()
            .unwrap()
            .localdb()
            .pkgs() // find exact match
            .find_satisfier(pkg.name.clone())
            .map(|p| vec![Package::from(p)]) // convert to owned
            .unwrap_or_default())
    }
}
