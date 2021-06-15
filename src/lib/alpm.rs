use std::sync::{Arc, Mutex};

use alpm::Alpm;
use lazy_static::lazy_static;

use crate::consts::*;
use crate::parser::GLOBAL_CONFIG;

lazy_static! {
    pub static ref GLOBAL_ALPM: Arc<Mutex<alpm::Alpm>> = {
        let alpm = Alpm::new(ROOT_PATH, PACMAN_DB_PATH).unwrap();
        for db in GLOBAL_CONFIG.sync_dbs() {
            alpm.register_syncdb(db.name.to_string(), db.sig_level)
                .unwrap();
        }
        Arc::new(Mutex::new(alpm))
    };
}
