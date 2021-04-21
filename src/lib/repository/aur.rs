use std::collections::HashMap;

use itertools::Itertools;
use raur::blocking::{Handle, Raur};

use crate::repository::{classify_package, sort_pkgs_mut, Repository};
use crate::types::*;

#[derive(Debug, Clone, Default)]
pub struct AurRepo {
    handler: Handle,
}

impl AurRepo {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Repository for AurRepo {
    fn find_package(&self, pkg: &str) -> Result<Vec<Package>> {
        // TODO error handling
        println!("aur searching for {}", pkg);
        let search_result = self
            .handler
            .search(pkg)
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.name)
            .collect_vec();
        let mut detailed_info = self
            .handler
            .info(&search_result)
            .unwrap()
            .into_iter()
            .map(Package::from)
            .filter(|p| {
                p.name() == pkg || p.provides().into_iter().any(|provide| provide.name == pkg)
            })
            .collect_vec();
        sort_pkgs_mut(&mut detailed_info, pkg);
        Ok(detailed_info)
    }

    fn find_packages(&self, pkgs: &[&str]) -> Result<HashMap<String, Vec<Package>>> {
        // TODO error handling
        println!("aur searching for {:?}", pkgs);
        // let search_result: HashMap<String, Vec<Package>> = pkgs.iter().map(|pkgname|self.handler.search(pkgname));
        let search_result = pkgs
            .iter()
            .map(|pkgname| self.handler.search(pkgname).unwrap_or_default()) // search candidates per package
            .flatten()
            .map(|p| p.name)
            .collect_vec();

        let mut detailed_info = self
            .handler
            .info(&search_result) // acquire detailed package info
            .unwrap()
            .into_iter()
            .map(Package::from) // convert to owned
            .map(|p| classify_package(p, pkgs)) // classify packages by requested package name
            .flatten() // collect into map
            .flatten() // filter None
            .into_group_map();

        for (pkgname, pkgs) in &mut detailed_info {
            sort_pkgs_mut(pkgs, pkgname);
        }

        Ok(detailed_info)
    }
}
