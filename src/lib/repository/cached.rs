use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};

use itertools::Itertools;

use crate::repository::Repository;
use crate::types::*;

#[derive(Debug, Clone)]
pub struct CachedRepository {
    inner: Arc<dyn Repository>,
    cache: Arc<RwLock<HashMap<Depend, Vec<Package>>>>,
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
    fn find_package(&self, pkg: &Depend) -> Result<Vec<Package>> {
        // search in cache first
        if let Some(hit) = self.cache.read().unwrap().get(pkg) {
            return Ok(hit.clone());
        }

        let missed = self.inner.find_package(pkg)?; // query missed packages
        self.cache
            .write()
            .unwrap()
            .insert(pkg.clone(), missed.clone()); // write back into cache
        Ok(missed)
    }

    fn find_packages(&self, pkgs: &[Depend]) -> Result<HashMap<Depend, Vec<Package>>> {
        // search in cache first
        let (mut hit_deps, missed_deps) = {
            let cache_read = self.cache.read().unwrap();
            let hit_deps: HashMap<Depend, Vec<Package>> = pkgs
                .iter()
                .filter_map(|dep| {
                    cache_read
                        .get(dep)
                        .map(|pkg| (dep.clone(), pkg.clone()))
                })
                .collect();
            let missed_deps = pkgs
                .iter()
                .filter(|pkgname| !hit_deps.contains_key(pkgname))
                .cloned()
                .collect_vec();
            (hit_deps, missed_deps)
        };

        // query missed packages
        let missed_packages = self.inner.find_packages(&missed_deps)?;

        // write back into cache
        {
            let mut cache_write = self.cache.write().unwrap();
            for (dep, packages) in &missed_packages {
                cache_write.insert(dep.clone(), packages.clone());
            }
        }

        // merge hit and missed set
        hit_deps.extend(missed_packages.into_iter());

        Ok(hit_deps)
    }
}
