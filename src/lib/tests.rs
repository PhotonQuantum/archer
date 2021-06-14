use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use itertools::Itertools;

pub use crate::prelude::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct PackageAssertion {
    name: String,
    version: DependVersion,
}

impl FromStr for PackageAssertion {
    type Err = !;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let dep = Depend::from_str(s).unwrap();
        Ok(PackageAssertion::new(dep.name, dep.version))
    }
}

#[macro_export]
macro_rules! assert_pkg {
    ($s: literal) => {
        PackageAssertion::from_str($s).unwrap()
    };
}

impl Display for PackageAssertion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "?[{}{}]", self.name, self.version)
    }
}

impl PackageAssertion {
    pub fn new(name: String, version: DependVersion) -> Self {
        PackageAssertion { name, version }
    }
    pub fn assert(&self, pkg: &Package) -> bool {
        self.name == pkg.name() && self.version.satisfied_by(&*pkg.version())
    }
}

pub fn test_pkg(
    name: String,
    version: Version,
    depends: Vec<Depend>,
    makedepends: Vec<Depend>,
    conflicts: Vec<Depend>,
    provides: Vec<Depend>,
) -> Package {
    Package::PacmanPackage(OwnedPacmanPackage {
        name,
        version,
        depends,
        makedepends,
        conflicts,
        provides,
        ..Default::default()
    })
}

#[macro_export]
macro_rules! dep {
    ($s: expr) => {
        Depend::from_str($s).unwrap()
    };
}

#[macro_export]
macro_rules! deps {
    ($($s: expr), *) => {
        vec![$(Depend::from_str($s).unwrap()),*]
    }
}

#[macro_export]
macro_rules! pkg {
    ($name: literal) => {
        test_pkg(
            String::from($name),
            Version(String::from("1.0.0")),
            vec![],
            vec![],
            vec![],
            vec![],
        )
    };
    ($name: literal, $ver: literal) => {
        test_pkg(
            String::from($name),
            Version(String::from($ver)),
            vec![],
            vec![],
            vec![],
            vec![],
        )
    };
    ($name: literal, $ver: literal, $depends: expr) => {
        test_pkg(
            String::from($name),
            Version(String::from($ver)),
            $depends,
            vec![],
            vec![],
            vec![],
        )
    };
    ($name: literal, $ver: literal, $depends: expr, $conflicts: expr) => {
        test_pkg(
            String::from($name),
            Version(String::from($ver)),
            $depends,
            vec![],
            $conflicts,
            vec![],
        )
    };
    ($name: literal, $ver: literal, $depends: expr, $makedepends: expr, $conflicts: expr, $provides: expr) => {
        test_pkg(
            String::from($name),
            Version(String::from($ver)),
            $depends,
            $makedepends,
            $conflicts,
            $provides,
        )
    };
}

fn must_pkg_order(tgt: &[&Package], pkgs: &[PackageAssertion]) {
    let info_prefix = format!(
        "AssertOrder({:?})",
        pkgs.iter().map(|s| s.to_string()).collect_vec()
    );
    println!("{}", info_prefix);
    let positions = pkgs.iter().enumerate().map(|(idx, pkg)| {
        (
            idx,
            tgt.iter()
                .enumerate()
                .find(|(_, candidate)| pkg.assert(candidate))
                .map(|(pos, _)| pos),
        )
    });
    assert!(
        positions
            .tuple_windows()
            .into_iter()
            .all(|((idx1, pos1), (idx2, pos2))| pos1.expect(&*format!(
                "{} {} not found",
                info_prefix,
                pkgs.get(idx1).unwrap()
            )) < pos2.expect(&*format!(
                "{} {} not found",
                info_prefix,
                pkgs.get(idx2).unwrap()
            ))),
        "{} assertion failed",
        info_prefix
    );
}

fn is_pkg_exists(pkgs: &[&Package], pkg: &PackageAssertion) -> bool {
    pkgs.iter().any(|candidate| pkg.assert(candidate))
}

pub enum PkgsAssertion {
    Order(Vec<PackageAssertion>),
    Exist(PackageAssertion),
    NotExist(PackageAssertion),
}

impl PkgsAssertion {
    pub fn assert(&self, pkgs: &[&Package]) {
        match self {
            PkgsAssertion::Order(s) => must_pkg_order(pkgs, s),
            PkgsAssertion::Exist(pkg) => {
                let info_prefix = format!("AssertExist({})", pkg);
                println!("{}", info_prefix);
                assert!(is_pkg_exists(pkgs, pkg), "{} assertion failed", info_prefix)
            }
            PkgsAssertion::NotExist(pkg) => {
                let info_prefix = format!("AssertNotExist({})", pkg);
                println!("{}", info_prefix);
                assert!(
                    !is_pkg_exists(pkgs, pkg),
                    "{} assertion failed",
                    info_prefix
                )
            }
        }
    }
}

#[macro_export]
macro_rules! asrt {
    ($s: literal < $($ss: literal)< *) => {
        PkgsAssertion::Order(vec![assert_pkg!($s), $(assert_pkg!($ss)),*])
    };
    ($s: literal) => {
        PkgsAssertion::Exist(assert_pkg!($s))
    };
    (!$s: literal) => {
        PkgsAssertion::NotExist(assert_pkg!($s))
    };
}

pub fn wait_pacman_lock() {
    let lock_path = PathBuf::from("/var/lib/pacman/db.lck");
    while lock_path.exists() {
        std::thread::sleep(Duration::from_secs(1));
    }
}
