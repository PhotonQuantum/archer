use super::*;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::borrow::Cow;
use std::str::FromStr;

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
