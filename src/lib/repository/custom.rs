use crate::repository::{sort_pkgs_mut, Repository};
use crate::types::*;

#[derive(Debug, Clone)]
pub struct CustomRepository {
    packages: Vec<Package>,
}

impl CustomRepository {
    pub fn new(packages: Vec<Package>) -> Self {
        Self { packages }
    }
}

impl Repository for CustomRepository {
    fn find_package(&self, pkg: &Depend) -> Result<Vec<Package>> {
        let mut result = self
            .packages
            .iter()
            .filter(|candidate| pkg.satisfied_by(candidate))
            .cloned()
            .collect();
        sort_pkgs_mut(&mut result, pkg);
        Ok(result)
    }
}
