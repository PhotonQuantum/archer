use online_scc_graph::Error as SCCGraphError;
use thiserror::Error;

use crate::types::*;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Eq, PartialEq, Error)]
pub enum ParseError {
    #[error("pacman - {0}")]
    PacmanError(String),
}

#[derive(Debug, Clone, Eq, PartialEq, Error)]
pub enum DependencyError {
    #[error("missing dependency - {0}")]
    MissingDependency(String),
    #[error("conflict dependency - {0}")]
    ConflictDependency(String),
    #[error("cyclic dependency - {0:?}")]
    CyclicDependency(Vec<ArcPackage>),
}

#[derive(Debug, Clone, Eq, PartialEq, Error)]
pub enum GraphError {
    #[error("internal scc graph error - {0}")]
    SCCGraphError(#[from] SCCGraphError),
    #[error("invalid node")]
    InvalidNode,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("pacman error: {0}")]
    PacmanError(#[from] alpm::Error),
    #[error("aur error: {0}")]
    AurError(#[from] raur::Error),
    #[error("parse error: {0}")]
    ParseError(#[from] ParseError),
    #[error("dependency error: {0}")]
    DependencyError(#[from] DependencyError),
    #[error("max recursion depth exceeded")]
    RecursionError,
    #[error("internal graph error: {0}")]
    GraphError(#[from] GraphError),
}
