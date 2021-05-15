use std::collections::HashMap;

use itertools::Itertools;
use raur::blocking::{Handle, Raur};
use rayon::prelude::*;

use crate::error::Result;
use crate::repository::{classify_package, Repository, sort_pkgs_mut};
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
    fn find_package(&self, pkg: &Depend) -> Result<Vec<Package>> {
        println!("aur searching for {}", pkg);
        let search_result = self
            .handler
            .search(&pkg.name)?
            .into_iter()
            .map(|p| p.name)
            .collect_vec();
        let mut detailed_info = self
            .handler
            .info(&search_result)?
            .into_iter()
            .map(Package::from)
            .filter(|candidate| pkg.satisfied_by(candidate))
            .collect();
        sort_pkgs_mut(&mut detailed_info, pkg);
        Ok(detailed_info)
    }

    fn find_packages(&self, pkgs: &[Depend]) -> Result<HashMap<Depend, Vec<Package>>> {
        println!("aur searching for {}", pkgs.iter().join(", "));
        // let search_result: HashMap<String, Vec<Package>> = pkgs.iter().map(|pkgname|self.handler.search(pkgname));
        let search_result: Vec<_> = pkgs
            .into_par_iter()
            .map(|dep| self.handler.search(&dep.name)) // search candidates per package Iter<Result<Vec<Package>>>
            .collect::<std::result::Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .map(|p| p.name)
            .collect();

        let mut detailed_info = self
            .handler
            .info(&search_result)? // acquire detailed package info
            .into_iter()
            .map(Package::from) // convert to owned
            .flat_map(|p| classify_package(p, pkgs)) // classify packages by requested package name
            .flatten() // filter None
            .into_group_map();

        for (pkgname, pkgs) in &mut detailed_info {
            sort_pkgs_mut(pkgs, pkgname);
        }

        Ok(detailed_info)
    }
}
