use lazy_static::lazy_static;

pub use pacman::Parser as PacmanParser;

pub mod pacman;

lazy_static! {
    pub static ref GLOBAL_CONFIG: PacmanParser = PacmanParser::with_default().unwrap();
}
