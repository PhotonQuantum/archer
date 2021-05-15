use crate::types::*;

#[derive(Clone, Eq, PartialEq, Hash)]
pub enum PlanAction {
    Install(Package),
    Build(Package),
    CopyToDest(Package),
}

