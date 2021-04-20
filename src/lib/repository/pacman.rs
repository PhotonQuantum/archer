use crate::alpm::GLOBAL_ALPM;
use crate::parser::pacman::SyncDB;
use crate::repository::Repository;
use crate::types::*;
use alpm::Alpm;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct PacmanRemote {
    alpm: Arc<Mutex<Alpm>>,
    cache: HashMap<String, Vec<Package>>
}

#[derive(Clone)]
pub struct PacmanLocal {
    alpm: Arc<Mutex<Alpm>>,
    cache: HashMap<String, Vec<Package>>
}

impl PacmanRemote {
    pub fn new() -> Self {
        Self { alpm: GLOBAL_ALPM.clone(), cache: Default::default() }
    }
}

impl Repository for PacmanRemote {
    fn find_package(&mut self, pkg: &str) -> Result<Vec<Package>> {
        if let Some(pkg) = self.cache.get(pkg) {
            Ok(pkg.to_vec())
        } else {
            let result: Vec<Package> = self
                .alpm
                .lock()
                .unwrap()
                .syncdbs()
                .find_satisfier(pkg)
                .map(|p| vec![p.into()])
                .unwrap_or_else(Vec::new);
            self.cache.insert(pkg.to_string(), result.clone());
            Ok(result)
        }
    }
}

impl PacmanLocal {
    pub fn new() -> Self {
        Self { alpm: GLOBAL_ALPM.clone(), cache: Default::default() }
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
