#![feature(box_patterns)]
#![feature(bool_to_option)]
#![feature(try_trait)]
#![feature(never_type)]
#![feature(map_into_keys_values)]
#![feature(hash_drain_filter)]

pub use utils::load_alpm;

pub mod alpm;
mod consts;
mod error;
pub mod parser;
pub mod repository;
pub mod resolver;
pub mod types;
mod utils;
