use crate::consts::*;
use crate::error::Result;
use crate::parser::PacmanParser;
use alpm::Alpm;
use fallible_iterator::Convert;

pub fn load_alpm() -> Result<Alpm> {
    let alpm = Alpm::new(ROOT_PATH, PACMAN_DB_PATH)?;
    let sync_dbs = PacmanParser::with_file(PACMAN_CONF_PATH)?.sync_dbs();
    for db in sync_dbs {
        alpm.register_syncdb(db.name, db.sig_level)?;
    }
    Ok(alpm)
}
