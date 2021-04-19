use crate::alpm::AlpmBuilder;
use crate::parser::pacman::SyncDB;
use crate::repository::Repository;
use crate::types::*;
use alpm::Alpm;
use async_trait::async_trait;

#[derive(Clone)]
pub struct PacmanRemote {
    alpm: AlpmBuilder,
}

#[derive(Clone)]
pub struct PacmanLocal {
    alpm: AlpmBuilder,
}

impl PacmanRemote {
    pub fn new(alpm: AlpmBuilder) -> Self {
        Self { alpm }
    }
}

impl Repository for PacmanRemote {
    fn find_package(&self, pkg: &str) -> Result<Vec<Package>> {
        Ok(self
            .alpm
            .build_sync()?
            .syncdbs()
            .find_satisfier(pkg)
            .map(|p| vec![p.into()])
            .unwrap_or_else(||vec![]))
    }
}

impl PacmanLocal {
    pub fn new(alpm: AlpmBuilder) -> Self {
        Self { alpm }
    }
}

impl Repository for PacmanLocal {
    fn find_package(&self, pkg: &str) -> Result<Vec<Package>> {
        Ok(self
            .alpm
            .build()?
            .localdb()
            .pkgs()
            .find_satisfier(pkg)
            .map(|p| vec![p.into()])
            .unwrap_or_else(||vec![]))
    }
}
