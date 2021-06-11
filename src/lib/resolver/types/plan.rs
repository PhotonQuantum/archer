use std::fmt::{Display, Formatter};

use itertools::Itertools;

use crate::types::*;

#[derive(Clone, Eq, PartialEq, Hash)]
pub enum PlanAction {
    Install(Package),
    InstallGroup(Vec<Package>),
    Build(Package),
    CopyToDest(Package),
}

impl Display for PlanAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanAction::Install(pkg) => write!(f, "Install({})", pkg.to_string()),
            PlanAction::Build(pkg) => write!(f, "Build({})", pkg.to_string()),
            PlanAction::CopyToDest(pkg) => write!(f, "CopyToDest({})", pkg.to_string()),
            PlanAction::InstallGroup(pkgs) => write!(
                f,
                "InstallGroup({})",
                pkgs.iter().map(ToString::to_string).join(", ")
            ),
        }
    }
}
