pub mod pacman;
use lazy_static::lazy_static;
pub use pacman::Parser as PacmanParser;

lazy_static! {
    pub static ref GLOBAL_CONFIG: PacmanParser = PacmanParser::with_default().unwrap();
}
