use super::*;
use std::fmt::{Display, Formatter};

macro_rules! option_owned {
    ($e: expr) => {
        $e.map(ToOwned::to_owned)
    };
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
