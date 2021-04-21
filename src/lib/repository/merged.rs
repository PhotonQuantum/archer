use crate::repository::Repository;
use crate::types::*;
use fallible_iterator::{convert, FallibleIterator};
use itertools::Itertools;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::prelude::rust_2015::Result::Ok;

#[derive(Default, Debug, Clone)]
pub struct MergedRepository {
    repos: Vec<Arc<Mutex<dyn Repository>>>,
}

impl MergedRepository {
    pub fn new(repos: Vec<Arc<Mutex<dyn Repository>>>) -> Self {
        Self { repos }
    }
}

impl Repository for MergedRepository {
    // NOTE
    // once there's valid response from a repo for each package, it won't be queried against succeeding repos

    fn find_package(&self, pkg: &str) -> Result<Vec<Package>> {
        convert(self.repos.iter().map(Ok)).fold(vec![], |mut acc, repo: &Arc<Mutex<dyn Repository>>|{
            if !acc.is_empty() {
                Ok(acc)
            } else {
                let result = repo.lock().unwrap().find_package(pkg)?;
                acc.extend(result);
                Ok(acc)
            }
        })
    }

    fn find_packages(&self, pkgs: &[&str]) -> Result<HashMap<String, Vec<Package>>> {
        let mut base = HashMap::new();
        for name in pkgs {
            base.insert(name.to_string(), vec![]);
        }
        convert(self.repos.iter().map(Ok)).fold(
            base,
            |mut acc, repo: &Arc<Mutex<dyn Repository>>| {
                let missed_pkgs = acc
                    .iter()
                    .filter(|(_, pkgs)| !pkgs.is_empty())
                    .map(|(name, _)| name.as_str())
                    .collect_vec();
                let mut result = repo.lock().unwrap().find_packages(&*missed_pkgs)?;
                acc.iter_mut().map(|(name, mut pkgs)| {
                    if let Some(new_pkgs) = result.remove(name) {
                        pkgs.extend(new_pkgs)
                    }
                });
                Ok(acc)
            },
        )
    }
}
