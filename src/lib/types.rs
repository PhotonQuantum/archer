use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Bound, RangeBounds};
use std::str::FromStr;
use std::sync::Arc;

pub use alpm::Package as PacmanPackage;
use alpm::{Dep, DepModVer};
use ranges::{Domain, GenericRange, Ranges};
pub use raur::Package as AurPackage;

use crate::repository::Repository;

pub type ArcRepo = Arc<dyn Repository>;

macro_rules! option_owned {
    ($e: expr) => {
        $e.map(ToOwned::to_owned)
    };
}

// TODO figure out a way to handle `epoch` field. see https://wiki.archlinux.org/index.php/PKGBUILD#Version
#[derive(Debug, Clone)]
pub struct Version(pub String);

impl From<&alpm::Ver> for Version {
    fn from(ver: &alpm::Ver) -> Self {
        Self(ver.to_string())
    }
}

impl Hash for Version {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // FIX: not reliable because of custom partial eq implementation
        self.0.hash(state)
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl<'a> AsRef<str> for Version {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        matches!(alpm::vercmp(self.as_ref(), other.as_ref()), Ordering::Equal)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(alpm::vercmp(self.as_ref(), other.as_ref()))
    }
}

impl Eq for Version {}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        alpm::vercmp(self.as_ref(), other.as_ref())
    }
}

impl Domain for Version {}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct DependVersion(pub Ranges<Version>);

const fn bound_of(bound: Bound<&Version>) -> Option<&Version> {
    match bound {
        Bound::Included(v) | Bound::Excluded(v) => Some(v),
        Bound::Unbounded => None,
    }
}

impl Display for DependVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0.as_slice().len() > 1 {
            write!(f, "multi_ranges") // archlinux doesn't support multi range constraint
        } else if let Some(range) = self.0.as_slice().first() {
            if range.is_full() {
                write!(f, "")
            } else if range.is_empty() {
                write!(f, " ∅")
            } else if range.is_singleton() {
                write!(f, " = {}", bound_of(range.start_bound()).unwrap())
            } else if !range.is_right_unbounded() && range.is_left_unbounded() {
                if range.is_right_closed() {
                    write!(f, " <= {}", bound_of(range.end_bound()).unwrap())
                } else {
                    write!(f, " < {}", bound_of(range.end_bound()).unwrap())
                }
            } else if !range.is_left_unbounded() && range.is_right_unbounded() {
                if range.is_left_closed() {
                    write!(f, " >= {}", bound_of(range.start_bound()).unwrap())
                } else {
                    write!(f, " > {}", bound_of(range.start_bound()).unwrap())
                }
            } else {
                write!(f, "double_ended_range") // archlinux doesn't support double end constraint in one string
            }
        } else {
            write!(f, ": ∅")
        }
    }
}

impl DependVersion {
    pub fn is_empty(&self) -> bool {
        !self.0.as_slice().iter().any(|range| !range.is_empty())
    }

    pub fn is_legal(&self) -> bool {
        !(self.is_empty() || self.0.as_slice().len() > 1)
    }

    pub fn split(&self) -> Vec<Self> {
        // TODO support <>
        if self.is_legal() {
            let range = self.0.as_slice().first().unwrap();
            if !range.is_left_unbounded() && !range.is_right_unbounded() {
                vec![
                    Self(Ranges::from(GenericRange::new_with_bounds(
                        range.start_bound().cloned(),
                        Bound::Unbounded,
                    ))),
                    Self(Ranges::from(GenericRange::new_with_bounds(
                        Bound::Unbounded,
                        range.end_bound().cloned(),
                    ))),
                ]
            } else {
                vec![self.clone()]
            }
        } else {
            vec![]
        }
    }

    pub fn intersect(&self, other: &Self) -> Self {
        Self(self.0.clone().intersect(other.0.clone()))
    }

    pub fn union(&self, other: &Self) -> Self {
        Self(self.0.clone().union(other.0.clone()))
    }

    pub fn contains(&self, other: &Self) -> bool {
        self.0.clone().intersect(other.0.clone()) == other.0
    }

    pub fn complement(&self) -> Self {
        Self(self.0.clone().invert())
    }

    pub fn satisfied_by(&self, target: &Version) -> bool {
        self.0.contains(target)
    }
}

impl<'a> From<alpm::DepModVer<'a>> for DependVersion {
    fn from(dep_ver: DepModVer<'a>) -> Self {
        Self(match dep_ver {
            DepModVer::Any => Ranges::full(),
            DepModVer::Eq(ver) => Ranges::from(Version::from(ver)),
            DepModVer::Ge(ver) => Ranges::from(Version::from(ver)..),
            DepModVer::Le(ver) => Ranges::from(..=Version::from(ver)),
            DepModVer::Gt(ver) => Ranges::from(GenericRange::new_greater_than(Version::from(ver))),
            DepModVer::Lt(ver) => Ranges::from(..Version::from(ver)),
        })
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct Depend {
    pub name: String,
    pub version: DependVersion,
}

impl Depend {
    pub fn satisfied_by(&self, candidate: &Package) -> bool {
        (candidate.name() == self.name && self.version.satisfied_by(&candidate.version()))
            || candidate
                .provides()
                .iter()
                .any(|provide| provide.name == self.name && self.version.contains(&provide.version))
    }
    pub fn split_ver(&self) -> Vec<Self> {
        self.version
            .split()
            .into_iter()
            .map(|ver| Self {
                name: self.name.clone(),
                version: ver,
            })
            .collect()
    }
}

impl Display for Depend {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.split_ver().as_slice() {
            [v] => write!(f, "{} {}", self.name, v.version),
            [v1, v2] => write!(f, "{} {} && {}", self.name, v1.version, v2.version),
            _ => write!(f, "{} {}", self.name, self.version),
        }
    }
}

impl From<&Package> for Depend {
    fn from(pkg: &Package) -> Self {
        Self {
            name: pkg.name().to_string(),
            version: DependVersion(Ranges::from(pkg.version().into_owned())),
        }
    }
}

macro_rules! split_cmp_op {
    ($s: ident, $sep: expr, $rel: expr) => {
        $s.split_once($sep).map(|(name, ver)| {
            (
                name.to_string(),
                DependVersion(Ranges::from($rel(Version(ver.to_string())))),
            )
        })
    };
}

impl FromStr for Depend {
    type Err = !;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        // TODO parse neq (!=? <>?)
        let (name, version) = split_cmp_op!(s, ">=", GenericRange::new_at_least)
            .or_else(|| split_cmp_op!(s, "<=", GenericRange::new_at_most))
            .or_else(|| split_cmp_op!(s, ">", GenericRange::new_greater_than))
            .or_else(|| split_cmp_op!(s, "<", GenericRange::new_less_than))
            .or_else(|| split_cmp_op!(s, "=", GenericRange::singleton))
            .unwrap_or((s.to_string(), DependVersion(Ranges::full())));
        Ok(Self { name, version })
    }
}

impl<'a> From<alpm::Dep<'a>> for Depend {
    fn from(dep: Dep<'a>) -> Self {
        Self {
            name: dep.name().to_string(),
            version: dep.depmodver().into(),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct PacmanFile {
    name: String,
    size: i64,
    mode: u32,
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct OwnedPacmanPackage {
    pub name: String,
    pub should_ignore: bool,
    pub filename: String,
    pub base: Option<String>,
    pub version: Version,
    pub origin: alpm::PackageFrom,
    pub desc: Option<String>,
    pub url: Option<String>,
    pub build_date: chrono::NaiveDateTime,
    pub install_date: Option<chrono::NaiveDateTime>,
    pub packager: Option<String>,
    pub md5sum: Option<String>,
    pub sha256sum: Option<String>,
    pub arch: Option<String>,
    pub size: i64,
    pub install_size: i64,
    pub reason: alpm::PackageReason,
    pub validation: alpm::PackageValidation,
    pub licenses: Vec<String>,
    pub groups: Vec<String>,
    pub depends: Vec<Depend>,
    pub optdepends: Vec<Depend>,
    pub checkdepends: Vec<Depend>,
    pub makedepends: Vec<Depend>,
    pub conflicts: Vec<Depend>,
    pub provides: Vec<Depend>,
    pub replaces: Vec<Depend>,
    pub files: Vec<PacmanFile>,
    pub backup: Vec<PacmanFile>,
    pub db: Option<String>,
    pub required_by: Vec<String>,
    pub optional_for: Vec<String>,
    pub base64_sig: Option<String>,
    pub has_scriptlet: bool,
}

impl Default for OwnedPacmanPackage {
    fn default() -> Self {
        Self {
            name: Default::default(),
            should_ignore: Default::default(),
            filename: Default::default(),
            base: Default::default(),
            version: Version(String::from("0")),
            origin: alpm::PackageFrom::File,
            desc: Default::default(),
            url: Default::default(),
            build_date: chrono::NaiveDateTime::from_timestamp(0, 0),
            install_date: Default::default(),
            packager: Default::default(),
            md5sum: Default::default(),
            sha256sum: Default::default(),
            arch: Default::default(),
            size: Default::default(),
            install_size: Default::default(),
            reason: alpm::PackageReason::Explicit,
            validation: alpm::PackageValidation::NONE,
            licenses: Default::default(),
            groups: Default::default(),
            depends: Default::default(),
            optdepends: Default::default(),
            checkdepends: Default::default(),
            makedepends: Default::default(),
            conflicts: Default::default(),
            provides: Default::default(),
            replaces: Default::default(),
            files: Default::default(),
            backup: Default::default(),
            db: Default::default(),
            required_by: Default::default(),
            optional_for: Default::default(),
            base64_sig: Default::default(),
            has_scriptlet: Default::default(),
        }
    }
}

impl Display for OwnedPacmanPackage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.name, self.version)
    }
}

impl From<PacmanPackage<'_>> for OwnedPacmanPackage {
    fn from(pkg: PacmanPackage) -> Self {
        Self::from(&pkg)
    }
}

impl From<&PacmanPackage<'_>> for OwnedPacmanPackage {
    fn from(pkg: &PacmanPackage) -> Self {
        Self {
            name: pkg.name().to_owned(),
            should_ignore: pkg.should_ignore(),
            filename: pkg.filename().to_owned(),
            base: option_owned!(pkg.base()),
            version: Version(pkg.version().to_string()),
            origin: pkg.origin(),
            desc: pkg.desc().map(ToOwned::to_owned),
            url: option_owned!(pkg.url()),
            build_date: chrono::NaiveDateTime::from_timestamp(pkg.build_date(), 0),
            install_date: pkg
                .install_date()
                .map(|dt| chrono::NaiveDateTime::from_timestamp(dt, 0)),
            packager: option_owned!(pkg.packager()),
            md5sum: option_owned!(pkg.md5sum()),
            sha256sum: option_owned!(pkg.sha256sum()),
            arch: option_owned!(pkg.arch()),
            size: pkg.size(),
            install_size: pkg.isize(),
            reason: pkg.reason(),
            validation: pkg.validation(),
            licenses: vec![],
            groups: vec![],
            depends: pkg.depends().iter().map(Depend::from).collect(),
            optdepends: pkg.optdepends().iter().map(Depend::from).collect(),
            checkdepends: pkg.checkdepends().iter().map(Depend::from).collect(),
            makedepends: pkg.makedepends().iter().map(Depend::from).collect(),
            conflicts: pkg.conflicts().iter().map(Depend::from).collect(),
            provides: pkg.provides().iter().map(Depend::from).collect(),
            replaces: pkg.replaces().iter().map(Depend::from).collect(),
            files: vec![],
            backup: vec![],
            db: pkg.db().map(|db| db.name().to_owned()),
            required_by: vec![],
            optional_for: vec![],
            base64_sig: option_owned!(pkg.base64_sig()),
            has_scriptlet: pkg.has_scriptlet(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Package {
    PacmanPackage(OwnedPacmanPackage),
    AurPackage(AurPackage),
}

impl PartialOrd for Package {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        use Package::*;
        (self.name() == other.name()).then(|| match self.version().cmp(&other.version()) {
            Ordering::Equal => match (self, other) {
                (PacmanPackage(_), AurPackage(_)) => Ordering::Greater,
                (AurPackage(_), PacmanPackage(_)) => Ordering::Less,
                _ => other.depends().len().cmp(&self.depends().len()),
            },
            ord => ord,
        })
    }
}

impl AsRef<Self> for Package {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl Display for Package {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let source = match self {
            Package::PacmanPackage(_) => "pacman",
            Package::AurPackage(_) => "aur",
        };
        write!(f, "[{}] {} {}", source, self.name(), self.version())
    }
}

impl PartialEq for Package {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name() && self.version() == other.version()
    }
}

impl Eq for Package {}

impl Hash for Package {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name().hash(state);
        self.version().hash(state);
    }
}

impl From<PacmanPackage<'_>> for Package {
    fn from(pkg: PacmanPackage) -> Self {
        Self::PacmanPackage(pkg.into())
    }
}

impl From<AurPackage> for Package {
    fn from(pkg: AurPackage) -> Self {
        Self::AurPackage(pkg)
    }
}

impl<'a> Package {
    pub fn name(&self) -> &str {
        match self {
            Package::PacmanPackage(pkg) => pkg.name.as_str(),
            Package::AurPackage(pkg) => pkg.name.as_str(),
        }
    }

    pub fn version(&'a self) -> Cow<'a, Version> {
        match self {
            Package::PacmanPackage(pkg) => Cow::Borrowed(&pkg.version),
            Package::AurPackage(pkg) => Cow::Owned(Version(pkg.version.clone())),
        }
    }

    pub fn description(&self) -> Option<&str> {
        match self {
            Package::PacmanPackage(pkg) => pkg.desc.as_deref(),
            Package::AurPackage(pkg) => pkg.description.as_deref(),
        }
    }

    pub fn url(&self) -> Option<&str> {
        match self {
            Package::PacmanPackage(pkg) => pkg.url.as_deref(),
            Package::AurPackage(pkg) => pkg.url.as_deref(),
        }
    }

    // TODO below: join same name into one DependVersion
    pub fn depends(&'a self) -> Cow<'a, Vec<Depend>> {
        match self {
            Package::PacmanPackage(pkg) => Cow::Borrowed(&pkg.depends),
            Package::AurPackage(pkg) => Cow::Owned(
                pkg.depends
                    .iter()
                    .map(|s| Depend::from_str(s).unwrap())
                    .collect(),
            ),
        }
    }

    // TODO below: join same name into one DependVersion
    pub fn make_depends(&'a self) -> Cow<'a, Vec<Depend>> {
        match self {
            Package::PacmanPackage(pkg) => Cow::Borrowed(&pkg.makedepends),
            Package::AurPackage(pkg) => Cow::Owned(
                pkg.make_depends
                    .iter()
                    .map(|s| Depend::from_str(s).unwrap())
                    .collect(),
            ),
        }
    }

    pub fn conflicts(&'a self) -> Cow<'a, Vec<Depend>> {
        match self {
            Package::PacmanPackage(pkg) => Cow::Borrowed(&pkg.conflicts),
            Package::AurPackage(pkg) => Cow::Owned(
                pkg.conflicts
                    .iter()
                    .map(|s| Depend::from_str(s).unwrap())
                    .collect(),
            ),
        }
    }

    pub fn provides(&'a self) -> Cow<'a, Vec<Depend>> {
        match self {
            Package::PacmanPackage(pkg) => Cow::Borrowed(&pkg.provides),
            Package::AurPackage(pkg) => Cow::Owned(
                pkg.provides
                    .iter()
                    .map(|s| Depend::from_str(s).unwrap())
                    .collect(),
            ),
        }
    }

    pub fn replaces(&'a self) -> Cow<'a, Vec<Depend>> {
        match self {
            Package::PacmanPackage(pkg) => Cow::Borrowed(&pkg.replaces),
            Package::AurPackage(pkg) => Cow::Owned(
                pkg.replaces
                    .iter()
                    .map(|s| Depend::from_str(s).unwrap())
                    .collect(),
            ),
        }
    }
}
