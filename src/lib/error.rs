use std::path::PathBuf;

use online_scc_graph::Error as SCCGraphError;
use rusoto_s3::{DeleteObjectError, GetObjectError, PutObjectError};
use thiserror::Error;

use crate::types::*;

pub type Result<T> = std::result::Result<T, Error>;

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

#[derive(Debug, Eq, PartialEq, Error)]
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
    #[error("io error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("unrecognized archive format")]
    ArchiveError,
    #[error("invalid package format")]
    PackageError,
    #[error("storage error: {0}")]
    StorageError(#[from] StorageError),
}
