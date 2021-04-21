use std::collections::HashMap;

use raur::blocking::{Handle, Raur};

use crate::repository::{Repository, sort_pkgs_mut};
use crate::types::*;

#[derive(Debug, Clone, Default)]
pub struct AurRepo {
    handler: Handle,
    cache: HashMap<String, Vec<Package>>,
}

impl AurRepo {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Repository for AurRepo {
    fn find_package(&mut self, pkg: &str) -> Result<Vec<Package>> {
        // TODO error handling
        if let Some(pkg) = self.cache.get(pkg) {
            Ok(pkg.to_vec())
        } else {
            println!("aur searching for {}", pkg);
            let result: Vec<_> = self
                .handler
                .search(pkg)
                .unwrap_or_default()
                .into_iter()
                .map(|p| p.name)
                .collect();
            let mut result: Vec<_> = self
                .handler
                .info(&result)
                .unwrap()
                .into_iter()
                .map(Package::from)
                .filter(|p| {
                    p.name() == pkg || p.provides().into_iter().any(|provide| provide.name == pkg)
                })
                .collect();
            sort_pkgs_mut(&mut result, pkg);
            self.cache.insert(pkg.to_string(), result.clone());
            Ok(result)
        }
    }
}
