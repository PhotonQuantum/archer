#![feature(box_patterns)]
#![feature(bool_to_option)]
#![feature(try_trait)]

pub mod alpm;
mod consts;
mod error;
pub mod parser;
pub mod repository;
pub mod resolver;
pub mod types;
mod utils;

pub use utils::load_alpm;
