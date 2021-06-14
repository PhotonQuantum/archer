use std::os::linux::fs::MetadataExt;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use alpm::Alpm;

use crate::consts::*;
use crate::error::{MakepkgError, Result};
use crate::parser::PacmanParser;

pub fn load_alpm() -> Result<Alpm> {
    let alpm = Alpm::new(ROOT_PATH, PACMAN_DB_PATH)?;
    let sync_dbs = PacmanParser::with_default()?.sync_dbs();
    for db in sync_dbs {
        alpm.register_syncdb(db.name, db.sig_level)?;
    }
    Ok(alpm)
}

// Get stdev of the nearest valid path of the given path (e.g. /home for /home/some/non/exist/path)
// NOTE: '..' in path is not handled
fn get_stdev(path: impl AsRef<Path>) -> Option<u64> {
    let target = path.as_ref();
    target
        .ancestors()
        .find_map(|try_path| try_path.metadata().ok().map(|path| path.st_dev()))
}

pub fn is_same_fs(path_1: impl AsRef<Path>, path_2: impl AsRef<Path>) -> bool {
    get_stdev(path_1)
        .and_then(|stdev_1| get_stdev(path_2).map(|stdev_2| stdev_1 == stdev_2))
        .unwrap_or(false)
}

pub fn unix_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

pub const fn map_makepkg_code(status_code: i32) -> Option<MakepkgError> {
    match status_code {
        0 | 13 => None,
        2 => Some(MakepkgError::Configuration),
        3 => Some(MakepkgError::InvalidOption),
        4 => Some(MakepkgError::InvalidFunction),
        5 => Some(MakepkgError::InviablePackage),
        6 => Some(MakepkgError::MissingSrc),
        7 => Some(MakepkgError::MissingPkgDir),
        10 => Some(MakepkgError::RunAsRoot),
        11 => Some(MakepkgError::NoPermission),
        12 => Some(MakepkgError::ParseError),
        15 => Some(MakepkgError::MissingProgram),
        16 => Some(MakepkgError::SignFailure),
        _ => Some(MakepkgError::Unknown),
    }
}
