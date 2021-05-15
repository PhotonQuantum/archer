use crate::types::*;
use maplit::hashset;
use petgraph::Graph;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::{Values, ValuesMut};

#[derive(Debug, Default, Clone)]
pub struct Context {
    pub packages: HashMap<String, Arc<Package>>,
    pub reasons: HashMap<Arc<Package>, HashSet<Arc<Package>>>,
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

impl Context {
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }

    pub fn append_reason(&mut self, pkg: Arc<Package>, reason: Arc<Package>) {
        self.reasons
            .entry(pkg)
            .and_modify(|reasons| {
                reasons.insert(reason.clone());
            })
            .or_insert(hashset!(reason));
    }

    pub fn append_reasons(&mut self, pkg: Arc<Package>, reason: HashSet<Arc<Package>>) {
        self.reasons
            .entry(pkg)
            .and_modify(|reasons| {
                reasons.extend(reason.clone());
            })
            .or_insert(reason);
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
        for (k, v) in other.reasons {
            self.reasons
                .entry(k)
                .and_modify(|reason| {
                    reason.extend(v.clone());
                })
                .or_insert(v); // borrow checker...
        }

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
        Self {
            packages: Default::default(),
            reasons: Default::default(),
            conflicts: Default::default(),
            provides: Default::default(),
        }
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
    pub fn insert(mut self, pkg: Arc<Package>, reason: HashSet<Arc<Package>>) -> Option<Self> {
        self.insert_mut(pkg, reason).then(|| self)
    }
    pub fn insert_mut(&mut self, pkg: Arc<Package>, reason: HashSet<Arc<Package>>) -> bool {
        // TODO unchecked insert
        if self.is_compatible(&*pkg) {
            let name = pkg.name().to_string();
            if let Some(existing) = self.packages.get(&name) {
                return existing.version() == pkg.version();
            }
            self.packages.insert(name, pkg.clone());
            self.reasons.insert(pkg.clone(), reason);

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

            true
        } else {
            false
        }
    }

    // TODO custom impl
    // This is actually SCC because we need to deal with loops
    pub fn topo_sort(&self) -> Vec<Arc<Package>> {
        let g = Graph::from(self);
        let sccs = petgraph::algo::kosaraju_scc(&g);
        sccs.into_iter()
            .flatten()
            .map(|node| g.node_weight(node).unwrap())
            .cloned()
            .collect()
    }
}

impl From<&Context> for Graph<Arc<Package>, String> {
    fn from(g: &Context) -> Self {
        let mut g_ = Graph::new();
        let mut map_pkg_idx = HashMap::new();

        g.reasons.keys().for_each(|pkg| {
            map_pkg_idx.insert(pkg.clone(), g_.add_node(pkg.clone()));
        });
        g.reasons.iter().for_each(|(pkg, reasons)| {
            let pkg_idx = map_pkg_idx.get(&*pkg).unwrap();
            reasons.iter().for_each(|reason| {
                let reason_idx = map_pkg_idx.get(reason).unwrap();
                g_.add_edge(*reason_idx, *pkg_idx, String::from(""));
            })
        });
        g_
    }
}
