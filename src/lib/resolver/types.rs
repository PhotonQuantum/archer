use std::collections::hash_map::{Values, ValuesMut};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};

use enumflags2::{bitflags, BitFlags};
use maplit::hashset;
use petgraph::Graph;

use crate::repository::Repository;
use crate::types::*;

type ArcRepo = Arc<dyn Repository>;

// TODO remove mutex cuz find_package(s) doesn't require mut now
#[derive(Clone)]
pub struct ResolvePolicy {
    pub from_repo: ArcRepo,
    pub skip_repo: ArcRepo,
    pub immortal_repo: ArcRepo,
    pub immortal_cache: Arc<RwLock<HashMap<Depend, bool>>>,
}

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DependChoice {
    Depends,
    MakeDepends,
}

pub type DependPolicy = BitFlags<DependChoice>;

pub fn always_depend(_: &Package) -> DependPolicy {
    BitFlags::from(DependChoice::Depends)
}

pub fn makedepend_if_aur(pkg: &Package) -> DependPolicy {
    match pkg {
        Package::PacmanPackage(_) => BitFlags::from(DependChoice::Depends),
        Package::AurPackage(_) => DependChoice::Depends | DependChoice::MakeDepends,
    }
}

impl ResolvePolicy {
    pub fn new(from_repo: ArcRepo, skip_repo: ArcRepo, immortal_repo: ArcRepo) -> Self {
        Self {
            from_repo,
            skip_repo,
            immortal_repo,
            immortal_cache: Arc::new(Default::default()),
        }
    }
    pub fn is_mortal_blade(&self, pkg: &Package) -> Result<bool> {
        let dep = Depend::from(&pkg.clone());
        if let Some(mortal_blade) = self.immortal_cache.read().unwrap().get(&dep) {
            return Ok(*mortal_blade);
        }
        let mortal_blade = self.immortal_repo.find_package(&dep).map(|immortals| {
            immortals
                .into_iter()
                .any(|immortal| immortal.version() != pkg.version())
        })?;
        self.immortal_cache
            .write()
            .unwrap()
            .insert(dep, mortal_blade);
        Ok(mortal_blade)
    }

    pub fn is_immortal(&self, pkg: &Package) -> Result<bool> {
        let dep = Depend::from(&pkg.clone());
        let immortal = self.immortal_repo.find_package(&dep).map(|immortals| {
            immortals
                .into_iter()
                .any(|immortal| immortal.version() == pkg.version())
        })?;
        Ok(immortal)
    }
}

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
            .map(|range| !range.intersect(&dep.version).is_empty())
            .unwrap_or(false)
    }

    pub fn satisfies(&self, dep: &Depend) -> bool {
        self.provides
            .get(&*dep.name)
            .map(|range| !range.intersect(&dep.version).is_empty())
            .unwrap_or(false)
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
            .map(|candidate| &**candidate == pkg)
            .unwrap_or(false)
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
                .map(|conflict| !conflict.intersect(&provide.version).is_empty())
                .unwrap_or(false)
        });

        let provides_conflict = pkg.conflicts().iter().any(|conflict| {
            self.provides
                .get(conflict.name.as_str())
                .map(|provide| !provide.intersect(&conflict.version).is_empty())
                .unwrap_or(false)
        });

        !(conflicts_conflict || provides_conflict)
    }
    pub fn insert(mut self, pkg: Arc<Package>, reason: HashSet<Arc<Package>>) -> Option<Self> {
        self.insert_mut(pkg, reason).then(|| self)
    }
    pub fn insert_mut(&mut self, pkg: Arc<Package>, reason: HashSet<Arc<Package>>) -> bool {
        // TODO unchecked insert
        if !self.is_compatible(&*pkg) {
            false
        } else {
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
        }
    }

    // TODO deal with cycles & custom impl
    pub fn topo_sort(&self) -> Vec<Arc<Package>> {
        let mut g = Graph::from(self);
        g.reverse();
        let sorted = petgraph::algo::toposort(&g, None).unwrap();
        sorted
            .into_iter()
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
