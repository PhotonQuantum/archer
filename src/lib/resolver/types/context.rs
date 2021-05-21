use std::collections::hash_map::{Values, ValuesMut};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::error::Result;
use crate::resolver::types::graph::{EdgeEffect, SCCGraph};
use crate::types::*;

#[derive(Debug, Default, Clone)]
pub struct Context {
    pub packages: HashMap<String, Arc<Package>>,
    pub graph: SCCGraph<Arc<Package>>,
    pub conflicts: HashMap<String, Arc<DependVersion>>,
    pub provides: HashMap<String, Arc<DependVersion>>,
}

impl PartialEq for Context {
    fn eq(&self, other: &Self) -> bool {
        self.packages == other.packages
    }
}

impl Eq for Context {}

impl Hash for Context {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for package in self.packages.values() {
            package.hash(state);
        }
    }
}

pub type MaybeCycle = Option<Vec<ArcPackage>>;

impl Context {
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }

    pub fn add_edge(
        &mut self,
        i: &Arc<Package>,
        j: &Arc<Package>,
    ) -> Result<EdgeEffect<Arc<Package>>> {
        self.graph.insert(i, j)
    }

    pub fn pkgs(&self) -> Values<String, Arc<Package>> {
        self.packages.values()
    }

    pub fn pkgs_mut(&mut self) -> ValuesMut<String, Arc<Package>> {
        self.packages.values_mut()
    }

    pub fn conflicts(&self, dep: &Depend) -> bool {
        self.conflicts
            .get(&*dep.name)
            .map_or(false, |range| !range.intersect(&dep.version).is_empty())
    }

    pub fn satisfies(&self, dep: &Depend) -> bool {
        self.provides
            .get(&*dep.name)
            .map_or(false, |range| !range.intersect(&dep.version).is_empty())
    }

    pub fn is_superset(&self, other: &[&Package]) -> bool {
        other.iter().all(|pkg| self.contains_exact(pkg))
    }

    pub fn union(mut self, other: Self) -> Option<Self> {
        for package in other.packages.values() {
            if !self.is_compatible(package) {
                return None;
            }
        }
        for (k, v2) in other.packages {
            if let Some(v1) = self.packages.get(&k) {
                if *v1 != v2 {
                    eprint!("FATAL: {} != {}", v1, v2);
                    return None;
                }
            }
            self.packages.insert(k, v2);
        }

        self.graph.merge(&other.graph).unwrap();

        for (k, v2) in other.provides {
            self.provides
                .entry(k)
                .and_modify(|v1| *v1 = Arc::new(v1.union(&*v2)))
                .or_insert(v2);
        }
        for (k, v2) in other.conflicts {
            self.conflicts
                .entry(k)
                .and_modify(|v1| *v1 = Arc::new(v1.intersect(&*v2)))
                .or_insert(v2);
        }
        Some(self)
    }
    pub fn new() -> Self {
        Default::default()
    }
    pub fn get(&self, name: &str) -> Option<&Package> {
        self.packages.get(name).map(|pkg| &**pkg)
    }
    pub fn contains_exact(&self, pkg: &Package) -> bool {
        self.packages
            .get(pkg.name())
            .map_or(false, |candidate| &**candidate == pkg)
    }
    pub fn is_compatible(&self, pkg: &Package) -> bool {
        if let Some(same_pkg_ver) = self
            .packages
            .get(pkg.name())
            .map(|old| old.version() == pkg.version())
        {
            return same_pkg_ver;
        };

        // let mut pkg_provides = vec![Depend::from(pkg)];
        let mut pkg_provides = vec![Depend::from(pkg)];
        pkg_provides.extend(pkg.provides().into_owned());
        let conflicts_conflict = pkg_provides.into_iter().any(|provide| {
            self.conflicts
                .get(provide.name.as_str())
                .map_or(false, |conflict| {
                    !conflict.intersect(&provide.version).is_empty()
                })
        });

        let provides_conflict = pkg.conflicts().iter().any(|conflict| {
            self.provides
                .get(conflict.name.as_str())
                .map_or(false, |provide| {
                    !provide.intersect(&conflict.version).is_empty()
                })
        });

        !(conflicts_conflict || provides_conflict)
    }
    pub fn insert(
        mut self,
        pkg: Arc<Package>,
        reasons: HashSet<Arc<Package>>,
    ) -> Option<(Self, MaybeCycle)> {
        self.insert_mut(pkg, reasons)
            .map(|maybe_cycle| (self, maybe_cycle))
    }

    // success(hascycle(cycle))
    pub fn insert_mut(
        &mut self,
        pkg: Arc<Package>,
        reasons: HashSet<Arc<Package>>,
    ) -> Option<MaybeCycle> {
        // TODO unchecked insert
        if self.is_compatible(&*pkg) {
            let name = pkg.name().to_string();
            if let Some(existing) = self.packages.get(&name) {
                return if existing.version() == pkg.version() {
                    Some(None)
                } else {
                    None
                };
            }
            self.packages.insert(name, pkg.clone());
            self.graph.add_node(pkg.clone());
            let cycle = reasons.iter().fold(None, |acc, reason| {
                let eff = self.graph.insert(&reason, &pkg).unwrap();
                if acc.is_none() {
                    if let EdgeEffect::NewEdge(Some(cycle)) = eff {
                        Some(cycle)
                    } else {
                        None
                    }
                } else {
                    None
                }
            });
            for reason in reasons {
                self.graph.insert(&reason, &pkg).unwrap();
            }

            let mut provides = pkg.provides().into_owned();
            provides.push(Depend::from(&*pkg));
            for provide in provides {
                let depend_version = if let Some(pkg) = self.provides.get(provide.name.as_str()) {
                    pkg.union(&provide.version)
                } else {
                    provide.version
                };
                self.provides.insert(provide.name, Arc::new(depend_version));
            }

            for conflict in pkg.conflicts().into_owned() {
                let conflict_version = if let Some(pkg) = self.conflicts.get(conflict.name.as_str())
                {
                    pkg.union(&conflict.version)
                } else {
                    conflict.version
                };
                self.conflicts
                    .insert(conflict.name, Arc::new(conflict_version));
            }

            Some(cycle)
        } else {
            None
        }
    }

    // TODO custom impl
    // This is actually SCC because we need to deal with loops
    pub fn strongly_connected_components(&self) -> Vec<Vec<&Arc<Package>>> {
        self.graph.strongly_connected_components(true)
    }
}
