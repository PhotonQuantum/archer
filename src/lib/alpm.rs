use std::sync::{Arc, Mutex};

use alpm::{Alpm, SigLevel};
use lazy_static::lazy_static;

use crate::consts::*;
use crate::error::Result;
use crate::parser::pacman::SyncDB;
use crate::parser::{PacmanParser, GLOBAL_CONFIG};

lazy_static! {
    pub static ref GLOBAL_ALPM: Arc<Mutex<alpm::Alpm>> = {
        let alpm = Alpm::new(ROOT_PATH, PACMAN_DB_PATH).unwrap();
        for db in GLOBAL_CONFIG.sync_dbs() {
            alpm.register_syncdb(db.name.to_string(), db.sig_level).unwrap();
        }
        Arc::new(Mutex::new(alpm))
    };
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
