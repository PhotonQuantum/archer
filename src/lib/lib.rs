#![deny(clippy::all)]
#![feature(box_patterns)]
#![feature(bool_to_option)]
#![feature(try_trait)]
#![feature(never_type)]
#![feature(hash_drain_filter)]
#![feature(bound_cloned)]

pub use utils::load_alpm;

#[cfg(test)]
#[macro_use]
mod tests;

pub mod alpm;
mod consts;
mod error;
pub mod parser;
pub mod repository;
pub mod resolver;
pub mod types;
mod utils;
