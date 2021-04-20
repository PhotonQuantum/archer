use crate::repository::Repository;
use crate::types::*;
use raur::blocking::{Handle, Raur};
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AurRepo {
    handler: Handle,
    cache: HashMap<String, Vec<Package>>,
}

impl AurRepo {
    pub fn new() -> Self {
        Self {
            handler: Default::default(),
            cache: Default::default(),
        }
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
                .unwrap_or(vec![])
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
            result.sort_unstable_by(|a, b| {
                if a.name() == pkg && b.name() != pkg {
                    Ordering::Less
                } else if a.name() != pkg && b.name() == pkg {
                    Ordering::Greater
                } else {
                    match a
                        .partial_cmp(b)
                        .unwrap_or_else(|| a.version().cmp(&b.version()))
                    {
                        Ordering::Less => Ordering::Greater,
                        Ordering::Greater => Ordering::Less,
                        ord => ord,
                    }
                }
            });
            self.cache.insert(pkg.to_string(), result.clone());
            Ok(result)
        }
    }
}
