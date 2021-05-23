#![deny(clippy::all)]
#![allow(
    clippy::module_name_repetitions,
    clippy::pub_enum_variant_names,
    clippy::wildcard_imports,
    clippy::default_trait_access
)]
#![feature(box_patterns)]
#![feature(bool_to_option)]
#![feature(never_type)]
#![feature(hash_drain_filter)]
#![feature(bound_cloned)]
#![feature(box_syntax)]
#![feature(destructuring_assignment)]

pub use utils::load_alpm;

#[cfg(test)]
#[macro_use]
mod tests;

pub mod alpm;
pub mod consts;
pub mod error;
pub mod parser;
pub mod prelude;
pub mod repository;
pub mod resolver;
pub mod types;
mod utils;
