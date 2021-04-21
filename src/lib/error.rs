use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Error)]
pub enum ParseError {
    #[error("pacman - {0}")]
    PacmanError(String),
}

#[derive(Debug, Clone, Error)]
pub enum DependencyError {
    #[error("missing dependency - {0}")]
    MissingDependency(String),
    #[error("conflict dependency - {0}")]
    DependencyConflict(String),
    #[error("cyclic dependency")]
    CyclicDependency,
}

#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("pacman error: {0}")]
    PacmanError(#[from] alpm::Error),
    // TODO doesn't implement Clone
    // #[error("aur error: {0}")]
    // AurError(#[from] raur::Error),
    #[error("parse error: {0}")]
    ParseError(#[from] ParseError),
    #[error("dependency error: {0}")]
    DependencyError(#[from] DependencyError),
    #[error("max recursion depth exceeded")]
    RecursionError,
    #[error("internal representation for None, and shouldn't be returned to end user. fire a bug if you see this.")]
    NoneError,
}

pub fn op_to_res<T>(v: Option<T>) -> Result<T> {
    v.ok_or(Error::NoneError)
}

pub fn resop_to_res<T>(v: Result<Option<T>>) -> Result<T> {
    v.and_then(op_to_res)
}

pub fn res_to_resop<T>(v: Result<T>) -> Result<Option<T>> {
    if let Err(Error::NoneError) = v {
        Ok(None)
    } else {
        v.map(Some)
    }
}
