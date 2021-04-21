use std::collections::HashMap;
use std::fmt::Debug;
use std::prelude::rust_2015::Result::Ok;
use std::sync::{Arc, RwLock};

use itertools::Itertools;

use crate::repository::Repository;
use crate::types::*;

#[derive(Debug, Clone)]
pub struct CachedRepository {
    inner: Arc<dyn Repository>,
    cache: Arc<RwLock<HashMap<String, Vec<Package>>>>,
}

impl CachedRepository {
    pub fn new(repo: impl Repository + 'static) -> Self {
        Self {
            inner: Arc::new(repo),
            cache: Arc::new(Default::default()),
        }
    }
}

impl Repository for CachedRepository {
    fn find_package(&self, pkg: &str) -> Result<Vec<Package>> {
        // search in cache first
        if let Some(hit) = self.cache.read().unwrap().get(pkg) {
            return Ok(hit.clone());
        }

        let missed = self.inner.find_package(pkg)?; // query missed packages
        self.cache
            .write()
            .unwrap()
            .insert(pkg.to_string(), missed.clone()); // write back into cache
        Ok(missed)
    }

    fn find_packages(&self, pkgs: &[&str]) -> Result<HashMap<String, Vec<Package>>> {
        // search in cache first
        let (mut hit_packages, missed_pkgnames) = {
            let cache_read = self.cache.read().unwrap();
            let hit_packages: HashMap<String, Vec<Package>> = pkgs
                .iter()
                .filter_map(|pkgname| {
                    cache_read
                        .get(*pkgname)
                        .map(|pkg| (pkgname.to_string(), pkg.clone()))
                })
                .collect();
            let missed_pkgnames = pkgs
                .iter()
                .filter(|pkgname| !hit_packages.contains_key(**pkgname))
                .copied()
                .collect_vec();
            (hit_packages, missed_pkgnames)
        };

        // query missed packages
        let missed_packages = self.inner.find_packages(&missed_pkgnames)?;

        // write back into cache
        {
            let mut cache_write = self.cache.write().unwrap();
            for (name, packages) in &missed_packages {
                cache_write.insert(name.to_string(), packages.clone());
            }
        }

        // merge hit and missed set
        hit_packages.extend(missed_packages.into_iter());

        Ok(hit_packages)
    }
}
