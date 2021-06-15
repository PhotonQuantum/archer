use lazy_static::lazy_static;

pub use pacman::*;

pub mod pacman;

#[cfg(test)]
mod tests;

lazy_static! {
    pub static ref GLOBAL_CONFIG: PacmanConf = PacmanConf::new().unwrap();
}
