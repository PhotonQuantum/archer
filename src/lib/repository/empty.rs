use std::collections::HashMap;

use crate::repository::Repository;
use crate::types::*;

#[derive(Copy, Clone, Debug, Default)]
pub struct EmptyRepository {}

impl EmptyRepository {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Repository for EmptyRepository {
    fn find_package(&self, pkg: &Depend) -> Result<Vec<Package>> {
        Ok(vec![])
    }

    fn find_packages(&self, pkgs: &[Depend]) -> Result<HashMap<Depend, Vec<Package>>> {
        Ok(HashMap::new())
    }
}
