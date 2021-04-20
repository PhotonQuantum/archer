use crate::error::{DependencyError, Result};
use crate::repository::Repository;
use crate::types::*;
use indexmap::{IndexMap, IndexSet};
use ranges::Ranges;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct ArcedIterator<I> {
    inner: Arc<Mutex<dyn Iterator<Item = I>>>,
}

impl<I> ArcedIterator<I> {
    pub fn new(iter: Arc<Mutex<dyn Iterator<Item = I>>>) -> Self {
        Self { inner: iter }
    }
}

impl<I> Iterator for ArcedIterator<I> {
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.lock().unwrap().next()
    }
}

#[derive(Debug, Clone)]
pub struct PackageWithParent {
    data: Package,
    // parent: Option<Arc<Box<PackageWithParent>>>
    parent: Option<Depend>,
}

impl PackageWithParent {
    pub fn with_parent(self, parent: Depend) -> Self {
        Self {
            data: self.data,
            parent: Some(parent),
        }
    }
}

impl Display for PackageWithParent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(parent) = &self.parent {
            // write!(f, "{} -> {}", parent, self.data)
            write!(f, "{} -> {}", parent, self.data)
        } else {
            write!(f, "{}", self.data)
        }
    }
}

impl PartialEq for PackageWithParent {
    fn eq(&self, other: &Self) -> bool {
        self.data.name() == other.data.name() && self.data.version() == other.data.version()
    }
}

impl Eq for PackageWithParent {}

impl Hash for PackageWithParent {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name().hash(state);
        self.version().hash(state);
    }
}

impl From<Package> for PackageWithParent {
    fn from(pkg: Package) -> Self {
        Self {
            data: pkg,
            parent: None,
        }
    }
}

impl AsRef<Package> for PackageWithParent {
    fn as_ref(&self) -> &Package {
        &self.data
    }
}

impl Deref for PackageWithParent {
    type Target = Package;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl PackageTrait for PackageWithParent {
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

// TODO refactor its ctor
#[derive(Default, Clone)]
pub struct ResolvePolicy {
    pub from_repo: Vec<Arc<Mutex<dyn Repository + Send>>>,
    pub skip_repo: Vec<Arc<Mutex<dyn Repository + Send>>>,
    pub immortal_repo: Vec<Arc<Mutex<dyn Repository + Send>>>,
}

#[derive(Debug, Default, Clone)]
pub struct DepList<T: PackageTrait> {
    pub packages: IndexMap<String, Arc<Box<T>>>,
    pub conflicts: HashMap<String, Arc<Box<DependVersion>>>,
    pub provides: HashMap<String, Arc<Box<DependVersion>>>,
}

impl<T: PackageTrait> PartialEq for DepList<T> {
    fn eq(&self, other: &Self) -> bool {
        self.packages == other.packages
    }
}

impl<T: PackageTrait> Eq for DepList<T> {}

impl<T: PackageTrait> Hash for DepList<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for (_, package) in &self.packages {
            package.hash(state);
        }
    }
}

impl<T: PackageTrait> DepList<T> {
    pub fn union(mut self, other: Self) -> Option<Self> {
        for (k, v2) in other.packages {
            if let Some(v1) = self.packages.get(&k) {
                if *v1 != v2 {
                    eprint!("FATAL: {} != {}", v1, v2);
                    return None;
                }
            }
            self.packages.insert(k, v2);
        }
        for (k, v2) in other.provides {
            let v = if let Some(v1) = self.provides.get(&k) {
                Arc::new(Box::new(v1.union(&**v2)))
            } else {
                v2
            };
            self.provides.insert(k, v);
        }
        for (k, v2) in other.conflicts {
            let v = if let Some(v1) = self.conflicts.get(&k) {
                Arc::new(Box::new(v1.intersect(&**v2)))
            } else {
                v2
            };
            self.conflicts.insert(k, v);
        }
        Some(self)
    }
    pub fn new() -> Self {
        Self {
            packages: Default::default(),
            conflicts: Default::default(),
            provides: Default::default(),
        }
    }
    pub fn get(&self, name: &str) -> Option<&T> {
        self.packages.get(name).map(|pkg| &***pkg)
    }
    pub fn contains_exact(&self, pkg: &T) -> bool {
        self.packages
            .get(pkg.name())
            .map(|candidate| &***candidate == pkg)
            .unwrap_or(false)
    }
    pub fn is_compatible(&self, pkg: &T) -> bool {
        let name_conflict = self.packages.get(pkg.name()).is_some();

        // let mut pkg_provides = vec![Depend::from(pkg)];
        let mut pkg_provides = vec![Depend::from(pkg.as_ref())];
        pkg_provides.extend(pkg.provides());
        let conflicts_conflict = pkg_provides.into_iter().any(|pkg| {
            self.conflicts
                .get(pkg.name.as_str())
                .map(|conflict| !conflict.intersect(&pkg.version).is_empty())
                .unwrap_or(false)
        });

        let provides_conflict = pkg.conflicts().iter().any(|conflict| {
            self.provides
                .get(conflict.name.as_str())
                .map(|provide| !provide.intersect(&conflict.version).is_empty())
                .unwrap_or(false)
        });

        !(name_conflict || conflicts_conflict || provides_conflict)
    }
    pub fn insert(mut self, pkg: Arc<Box<T>>) -> Option<Self> {
        self.insert_mut(pkg).then(|| self)
    }
    pub fn insert_mut(&mut self, pkg: Arc<Box<T>>) -> bool {
        // TODO unchecked insert
        if !self.is_compatible(&**pkg) {
            false
        } else {
            let name = pkg.name().to_string();
            self.packages.insert(name, pkg.clone());

            let mut provides = pkg.provides();
            provides.push(Depend::from((&**pkg).as_ref()));
            for provide in provides {
                let depend_version = if let Some(pkg) = self.provides.get(provide.name.as_str()) {
                    pkg.union(&provide.version)
                } else {
                    provide.version
                };
                self.provides
                    .insert(provide.name, Arc::new(Box::new(depend_version)));
            }

            for conflict in pkg.conflicts() {
                let conflict_version = if let Some(pkg) = self.conflicts.get(conflict.name.as_str())
                {
                    pkg.union(&conflict.version)
                } else {
                    conflict.version
                };
                self.conflicts
                    .insert(conflict.name, Arc::new(Box::new(conflict_version)));
            }

            true
        }
    }
}
