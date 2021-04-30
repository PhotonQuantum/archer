use std::collections::hash_map::{Values, ValuesMut};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::{Arc, RwLock};

use itertools::Itertools;
use maplit::hashset;
use petgraph::Graph;

use crate::repository::Repository;
use crate::types::*;

type ArcRepo = Arc<dyn Repository>;

#[derive(Debug, Clone)]
pub struct PackageNode {
    data: Package,
    // parent: Option<Arc<Box<PackageWithParent>>>
    reason: Vec<Depend>,
}

impl PackageNode {
    pub fn add_parent(mut self, parent: Depend) -> Self {
        self.reason.push(parent);
        self
    }
}

impl Display for PackageNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.reason.is_empty() {
            write!(f, "{}", self.data)
        } else {
            write!(
                f,
                "({}) -> {}",
                self.reason.iter().map(|r| r.to_string()).join(","),
                self.data
            )
        }
    }
}

impl PartialEq for PackageNode {
    fn eq(&self, other: &Self) -> bool {
        self.data.name() == other.data.name() && self.data.version() == other.data.version()
    }
}

impl Eq for PackageNode {}

impl Hash for PackageNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name().hash(state);
        self.version().hash(state);
    }
}

impl From<Package> for PackageNode {
    fn from(pkg: Package) -> Self {
        Self {
            data: pkg,
            reason: vec![],
        }
    }
}

impl From<&Package> for PackageNode {
    fn from(pkg: &Package) -> Self {
        Self {
            data: pkg.clone(),
            reason: vec![],
        }
    }
}

impl AsRef<Package> for PackageNode {
    fn as_ref(&self) -> &Package {
        &self.data
    }
}

impl Deref for PackageNode {
    type Target = Package;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

// TODO don't duplicate this
impl PackageTrait for PackageNode {
    fn name(&self) -> &str {
        self.data.name()
    }

    fn version(&self) -> Version {
        self.data.version()
    }

    fn description(&self) -> Option<&str> {
        self.data.description()
    }

    fn url(&self) -> Option<&str> {
        self.data.url()
    }

    fn dependencies(&self) -> Vec<Depend> {
        self.data.dependencies()
    }

    fn conflicts(&self) -> Vec<Depend> {
        self.data.conflicts()
    }

    fn provides(&self) -> Vec<Depend> {
        self.data.provides()
    }

    fn replaces(&self) -> Vec<Depend> {
        self.data.replaces()
    }
}

impl PackageTrait for &PackageNode {
    fn name(&self) -> &str {
        self.data.name()
    }

    fn version(&self) -> Version {
        self.data.version()
    }

    fn description(&self) -> Option<&str> {
        self.data.description()
    }

    fn url(&self) -> Option<&str> {
        self.data.url()
    }

    fn dependencies(&self) -> Vec<Depend> {
        self.data.dependencies()
    }

    fn conflicts(&self) -> Vec<Depend> {
        self.data.conflicts()
    }

    fn provides(&self) -> Vec<Depend> {
        self.data.provides()
    }

    fn replaces(&self) -> Vec<Depend> {
        self.data.replaces()
    }
}

// TODO remove mutex cuz find_package(s) doesn't require mut now
#[derive(Clone)]
pub struct ResolvePolicy {
    pub from_repo: ArcRepo,
    pub skip_repo: ArcRepo,
    pub immortal_repo: ArcRepo,
    pub immortal_cache: Arc<RwLock<HashMap<Depend, bool>>>,
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
    pub fn is_mortal_blade(&self, pkg: impl PackageTrait) -> Result<bool> {
        let dep = Depend::from(pkg.clone());
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

    pub fn is_immortal(&self, pkg: impl PackageTrait) -> Result<bool> {
        let dep = Depend::from(pkg.clone());
        let immortal = self.immortal_repo.find_package(&dep).map(|immortals| {
            immortals
                .into_iter()
                .any(|immortal| immortal.version() == pkg.version())
        })?;
        Ok(immortal)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Context<T: PackageTrait> {
    pub packages: HashMap<String, Arc<T>>,
    pub reasons: HashMap<Arc<T>, HashSet<Arc<T>>>,
    pub conflicts: HashMap<String, Arc<DependVersion>>,
    pub provides: HashMap<String, Arc<DependVersion>>,
}

impl<T: PackageTrait> PartialEq for Context<T> {
    fn eq(&self, other: &Self) -> bool {
        self.packages == other.packages
    }
}

impl<T: PackageTrait> Eq for Context<T> {}

impl<T: PackageTrait> Hash for Context<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for package in self.packages.values() {
            package.hash(state);
        }
    }
}

impl<T: PackageTrait> Context<T> {
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }

    pub fn append_reason(&mut self, pkg: Arc<T>, reason: Arc<T>) {
        self.reasons
            .entry(pkg)
            .and_modify(|reasons| {
                reasons.insert(reason.clone());
            })
            .or_insert(hashset!(reason));
    }

    pub fn append_reasons(&mut self, pkg: Arc<T>, reason: HashSet<Arc<T>>) {
        self.reasons
            .entry(pkg)
            .and_modify(|reasons| {
                reasons.extend(reason.clone());
            })
            .or_insert(reason);
    }

    pub fn pkgs(&self) -> Values<String, Arc<T>> {
        self.packages.values()
    }

    pub fn pkgs_mut(&mut self) -> ValuesMut<String, Arc<T>> {
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

    pub fn is_superset(&self, other: &[&T]) -> bool {
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
    pub fn get(&self, name: &str) -> Option<&T> {
        self.packages.get(name).map(|pkg| &**pkg)
    }
    pub fn contains_exact(&self, pkg: &T) -> bool {
        self.packages
            .get(pkg.name())
            .map(|candidate| &**candidate == pkg)
            .unwrap_or(false)
    }
    pub fn is_compatible(&self, pkg: &T) -> bool {
        if let Some(same_pkg_ver) = self
            .packages
            .get(pkg.name())
            .map(|old| old.version() == pkg.version())
        {
            return same_pkg_ver;
        };

        // let mut pkg_provides = vec![Depend::from(pkg)];
        let mut pkg_provides = vec![Depend::from(pkg.as_ref())];
        pkg_provides.extend(pkg.provides());
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
    pub fn insert(mut self, pkg: Arc<T>, reason: HashSet<Arc<T>>) -> Option<Self> {
        self.insert_mut(pkg, reason).then(|| self)
    }
    pub fn insert_mut(&mut self, pkg: Arc<T>, reason: HashSet<Arc<T>>) -> bool {
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

            let mut provides = pkg.provides();
            provides.push(Depend::from((&*pkg).as_ref()));
            for provide in provides {
                let depend_version = if let Some(pkg) = self.provides.get(provide.name.as_str()) {
                    pkg.union(&provide.version)
                } else {
                    provide.version
                };
                self.provides.insert(provide.name, Arc::new(depend_version));
            }

            for conflict in pkg.conflicts() {
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
}

impl<T: PackageTrait> From<&Context<T>> for Graph<Arc<T>, String> {
    fn from(g: &Context<T>) -> Self {
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
