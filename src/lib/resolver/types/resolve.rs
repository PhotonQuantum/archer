use crate::types::*;
use crate::error::Result;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use enumflags2::{bitflags, BitFlags};

#[derive(Clone)]
pub struct ResolvePolicy {
    pub from_repo: ArcRepo,
    pub skip_repo: ArcRepo,
    pub immortal_repo: ArcRepo,
    pub immortal_cache: Arc<RwLock<HashMap<Depend, bool>>>,
}

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DependChoice {
    Depends,
    MakeDepends,
}

pub type DependPolicy = BitFlags<DependChoice>;

pub fn always_depend(_: &Package) -> DependPolicy {
    BitFlags::from(DependChoice::Depends)
}

pub fn makedepend_if_aur(pkg: &Package) -> DependPolicy {
    match pkg {
        Package::PacmanPackage(_) => BitFlags::from(DependChoice::Depends),
        Package::AurPackage(_) => DependChoice::Depends | DependChoice::MakeDepends,
    }
}

impl ResolvePolicy {
    pub fn new(from_repo: ArcRepo, skip_repo: ArcRepo, immortal_repo: ArcRepo) -> Self {
        Self {
            from_repo,
            skip_repo,
            immortal_repo,
            immortal_cache: Arc::new(Default::default()),
        }
    }
    pub fn is_mortal_blade(&self, pkg: &Package) -> Result<bool> {
        let dep = Depend::from(&pkg.clone());
        if let Some(mortal_blade) = self.immortal_cache.read().unwrap().get(&dep) {
            return Ok(*mortal_blade);
        }
        let mortal_blade = self.immortal_repo.find_package(&dep).map(|immortals| {
            immortals
                .into_iter()
                .any(|immortal| immortal.version() != pkg.version())
        })?;
        self.immortal_cache
            .write()
            .unwrap()
            .insert(dep, mortal_blade);
        Ok(mortal_blade)
    }

    pub fn is_immortal(&self, pkg: &Package) -> Result<bool> {
        let dep = Depend::from(&pkg.clone());
        let immortal = self.immortal_repo.find_package(&dep).map(|immortals| {
            immortals
                .into_iter()
                .any(|immortal| immortal.version() == pkg.version())
        })?;
        Ok(immortal)
    }
}
