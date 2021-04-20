use crate::consts::*;
use crate::error::Result;
use crate::parser::pacman::SyncDB;
use crate::parser::PacmanParser;
use alpm::Alpm;
use lazy_static::lazy_static;

lazy_static!{
    static ref GLOBAL_ALPM_LOCAL: alpm::Alpm = Alpm::new(ROOT_PATH, PACMAN_DB_PATH).unwrap()
}

#[derive(Clone)]
pub struct AlpmBuilder {
    sync_dbs: Vec<SyncDB>,
}

impl AlpmBuilder {
    pub fn new(config: &PacmanParser) -> Self {
        Self {
            sync_dbs: config.sync_dbs(),
        }
    }

    pub fn build(&self) -> Result<Alpm> {
        Ok(Alpm::new(ROOT_PATH, PACMAN_DB_PATH)?)
    }

    pub fn build_sync(&self) -> Result<Alpm> {
        let alpm = Alpm::new(ROOT_PATH, PACMAN_DB_PATH)?;
        for db in &self.sync_dbs {
            alpm.register_syncdb(db.name.to_string(), db.sig_level)?;
        }
        Ok(alpm)
    }
}
