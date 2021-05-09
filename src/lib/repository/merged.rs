use std::collections::HashMap;
use std::sync::Arc;

use itertools::Itertools;

use crate::repository::Repository;
use crate::types::*;

#[derive(Default, Debug, Clone)]
pub struct MergedRepository {
    repos: Vec<Arc<dyn Repository>>,
}

impl MergedRepository {
    pub fn new(repos: Vec<Arc<dyn Repository>>) -> Self {
        Self { repos }
    }
}

impl Repository for MergedRepository {
    // NOTE
    // once there's valid response from a repo for each package, it won't be queried against succeeding repos

    fn find_package(&self, pkg: &Depend) -> Result<Vec<Package>> {
        self.repos
            .iter()
            .map(|repo| repo.find_package(pkg))
            .find(|result| {
                if let Ok(pkgs) = result {
                    !pkgs.is_empty()
                } else {
                    true
                }
            })
            .unwrap_or_else(|| Ok(vec![]))
    }

    fn find_packages(&self, pkgs: &[Depend]) -> Result<HashMap<Depend, Vec<Package>>> {
        let mut base = HashMap::new();
        for name in pkgs {
            base.insert(name.clone().clone(), vec![]);
        }

        // use try_fold instead of try_find here because we want Vec<Package> instead of ArcRepo
        self.repos.iter().try_fold(base, |mut acc, repo| {
            let missed_pkgs = acc
                .iter()
                .filter(|(_, pkgs)| pkgs.is_empty())
                .map(|(name, _)| name)
                .cloned()
                .collect_vec();
            if !missed_pkgs.is_empty() {
                let mut result = repo.find_packages(missed_pkgs.as_ref())?;
                for (name, pkgs) in &mut acc {
                    if let Some(new_pkgs) = result.remove(name) {
                        pkgs.extend(new_pkgs)
                    }
                }
            }
            Ok(acc)
        })
    }
}
