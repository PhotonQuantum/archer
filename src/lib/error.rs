use std::path::PathBuf;

use online_scc_graph::Error as SCCGraphError;
use rusoto_s3::{DeleteObjectError, GetObjectError, PutObjectError};
use thiserror::Error;

use crate::types::*;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Eq, PartialEq, Error)]
pub enum GpgError {
    #[error("Unknown fatal error")]
    Unknown,
    #[error("At least a signature was bad")]
    BadSignature,
    #[error("Interrupted by signal")]
    Signal,
}

#[derive(Debug, Eq, PartialEq, Error)]
pub enum MakepkgError {
    #[error("Unknown cause of failure")]
    Unknown,
    #[error("Error in configuration file")]
    Configuration,
    #[error("User specified an invalid option. This is likely an internal error. Fire a bug report if you meet one.")]
    InvalidOption,
    #[error("Error in user-supplied function in PKGBUILD")]
    InvalidFunction,
    #[error("Failed to create a viable package")]
    InviablePackage,
    #[error("A source or auxiliary file specified in the PKGBUILD is missing")]
    MissingSrc,
    #[error("The PKGDIR is missing")]
    MissingPkgDir,
    #[error("User attempted to run makepkg as root")]
    RunAsRoot,
    #[error("User lacks permissions to build or install to a given location")]
    NoPermission,
    #[error("Error parsing PKGBUILD")]
    ParseError,
    #[error("Programs necessary to run makepkg are missing")]
    MissingProgram,
    #[error("Specified GPG key does not exist or failed to sign package")]
    SignFailure,
    #[error("Interrupted by signal")]
    Signal,
}

#[derive(Debug, Eq, PartialEq, Error)]
pub enum CommandError {
    #[error("unknown command")]
    Unknown,
    #[error("pacman")]
    Pacman,
    #[error("makepkg: {0}")]
    Makepkg(MakepkgError),
    #[error("chown")]
    Chown,
    #[error("gpg: {0}")]
    Gpg(GpgError),
    #[error("pacman-key")]
    PacmanKey,
}

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("io error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("command execution failure: {0}")]
    CommandError(#[from] CommandError),
    #[error("unable to acquire lock")]
    LockError,
}

#[derive(Debug, Error)]
pub enum S3Error {
    #[error("get error: {0}")]
    GetError(#[from] rusoto_core::RusotoError<GetObjectError>),
    #[error("put error: {0}")]
    PutError(#[from] rusoto_core::RusotoError<PutObjectError>),
    #[error("delete error: {0}")]
    DeleteError(#[from] rusoto_core::RusotoError<DeleteObjectError>),
    #[error("builder error: {0}")]
    BuilderError(String),
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("invalid path: {0}")]
    InvalidPath(PathBuf),
    #[error("io error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("file exists: {0}")]
    FileExists(PathBuf),
    #[error("file doesn't exist: {0}")]
    FileNotExists(PathBuf),
    #[error("storage is in an inconsistent state")]
    Conflict,
    #[error("s3 error: {0}")]
    S3Error(#[from] S3Error),
    #[error("json error: {0}")]
    JSONError(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("pacman: {0}")]
    PacmanError(String),
    #[error("command execution failure: {0}")]
    CommandError(CommandError),
    #[error("io error: {0}")]
    IOError(#[from] std::io::Error),
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
    #[error("io error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("unrecognized archive format")]
    ArchiveError,
    #[error("invalid package format")]
    PackageError,
    #[error("storage error: {0}")]
    StorageError(#[from] StorageError),
    #[error("build error: {0}")]
    BuildError(#[from] BuildError),
}
