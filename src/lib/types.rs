use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::str::FromStr;

pub use alpm::Package as PacmanPackage;
use alpm::{Dep, DepModVer};
use ranges::{Domain, GenericRange, Ranges};
pub use raur::Package as AurPackage;

use crate::error::Error;

macro_rules! option_owned {
    ($e: expr) => {
        $e.map(ToOwned::to_owned)
    };
}

pub type Result<T> = std::result::Result<T, Error>;

// TODO figure out a way to handle `epoch` field. see https://wiki.archlinux.org/index.php/PKGBUILD#Version
#[derive(Debug, Clone)]
pub struct Version(String);

impl From<&alpm::Ver> for Version {
    fn from(ver: &alpm::Ver) -> Self {
        Version(ver.to_string())
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

impl DependVersion {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn intersect(&self, other: &DependVersion) -> DependVersion {
        DependVersion(self.0.clone().intersect(other.0.clone()))
    }

    pub fn union(&self, other: &DependVersion) -> DependVersion {
        DependVersion(self.0.clone().union(other.0.clone()))
    }

    pub fn contains(&self, other: &DependVersion) -> bool {
        self.0.clone().intersect(other.0.clone()) == other.0
    }

    pub fn complement(&self) -> DependVersion {
        DependVersion(self.0.clone().invert())
    }

    pub fn satisfied_by(&self, target: &Version) -> bool {
        self.0.contains(target)
    }
}

impl<'a> From<alpm::DepModVer<'a>> for DependVersion {
    fn from(dep_ver: DepModVer<'a>) -> Self {
        DependVersion(match dep_ver {
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
}

impl Display for Depend {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // TODO into arch version format
        write!(f, "{} {}", self.name, self.version.0)
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
    name: String,
    should_ignore: bool,
    filename: String,
    base: Option<String>,
    version: Version,
    origin: alpm::PackageFrom,
    desc: Option<String>,
    url: Option<String>,
    build_date: chrono::NaiveDateTime,
    install_date: Option<chrono::NaiveDateTime>,
    packager: Option<String>,
    md5sum: Option<String>,
    sha256sum: Option<String>,
    arch: Option<String>,
    size: i64,
    install_size: i64,
    reason: alpm::PackageReason,
    validation: alpm::PackageValidation,
    licenses: Vec<String>,
    groups: Vec<String>,
    depends: Vec<Depend>,
    optdepends: Vec<Depend>,
    checkdepends: Vec<Depend>,
    makedepends: Vec<Depend>,
    conflicts: Vec<Depend>,
    provides: Vec<Depend>,
    replaces: Vec<Depend>,
    files: Vec<PacmanFile>,
    backup: Vec<PacmanFile>,
    db: Option<String>,
    required_by: Vec<String>,
    optional_for: Vec<String>,
    base64_sig: Option<String>,
    has_scriptlet: bool,
}

impl Display for OwnedPacmanPackage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.name, self.version)
    }
}

impl From<PacmanPackage<'_>> for OwnedPacmanPackage {
    fn from(pkg: PacmanPackage) -> Self {
        OwnedPacmanPackage::from(&pkg)
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

impl AsRef<Package> for Package {
    fn as_ref(&self) -> &Package {
        self
    }
}

impl Display for Package {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let source = match self {
            Package::PacmanPackage(_) => "pacman",
            Package::AurPackage(_) => "aur"
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
        Package::PacmanPackage(pkg.into())
    }
}

impl From<AurPackage> for Package {
    fn from(pkg: AurPackage) -> Self {
        Package::AurPackage(pkg)
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
