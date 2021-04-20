pub mod pacman;
pub use pacman::Parser as PacmanParser;
use lazy_static::lazy_static;

lazy_static!{
    pub static ref GLOBAL_CONFIG: PacmanParser = PacmanParser::with_default().unwrap();
}