use crate::repository::Repository;
use crate::types::*;

#[derive(Debug, Clone)]
pub struct CustomRepository {
    packages: Vec<Package>,
}

impl CustomRepository {
    pub fn new(packages: Vec<Package>) -> Self {
        CustomRepository { packages }
    }
}

impl Repository for CustomRepository {
    fn find_package(&self, pkg: &Depend) -> Result<Vec<Package>> {
        Ok(self
            .packages
            .iter()
            .filter(|candidate| pkg.satisfied_by(candidate))
            .cloned()
            .collect())
    }
}
