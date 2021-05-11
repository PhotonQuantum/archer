use alpm::SigLevel;
use configparser::ini::Ini;

use crate::consts::*;
use crate::error::ParseError;

pub struct Parser {
    config: Ini,
}

#[derive(Clone)]
pub struct SyncDB {
    pub name: String,
    pub sig_level: alpm::SigLevel,
    pub server: Option<String>,
}

fn parse_siglevel<'a>(content: impl IntoIterator<Item = &'a str>) -> Option<SigLevel> {
    let mut siglevel = SigLevel::NONE;
    for signal in content {
        let (mut package, mut database) = (true, true);
        let flag = signal
            .strip_prefix("Package")
            .map(|s| {
                database = false;
                s
            })
            .or_else(|| {
                signal.strip_prefix("Database").map(|s| {
                    package = false;
                    s
                })
            })
            .unwrap_or(signal);
        match flag {
            "Never" => {
                if package {
                    siglevel.remove(SigLevel::PACKAGE);
                }
                if database {
                    siglevel.remove(SigLevel::DATABASE);
                }
            }
            "Optional" => {
                if package {
                    siglevel.insert(SigLevel::PACKAGE | SigLevel::PACKAGE_OPTIONAL);
                }
                if database {
                    siglevel.insert(SigLevel::DATABASE | SigLevel::DATABASE_OPTIONAL);
                }
            }
            "Required" => {
                if package {
                    siglevel.insert(SigLevel::PACKAGE);
                    siglevel.remove(SigLevel::PACKAGE_OPTIONAL);
                }
                if database {
                    siglevel.insert(SigLevel::DATABASE);
                    siglevel.remove(SigLevel::DATABASE_OPTIONAL);
                }
            }
            "TrustedOnly" => {
                if package {
                    siglevel.remove(SigLevel::PACKAGE_MARGINAL_OK | SigLevel::PACKAGE_UNKNOWN_OK);
                }
                if database {
                    siglevel.remove(SigLevel::DATABASE_MARGINAL_OK | SigLevel::DATABASE_UNKNOWN_OK);
                }
            }
            "TrustAll" => {
                if package {
                    siglevel.insert(SigLevel::PACKAGE_MARGINAL_OK | SigLevel::PACKAGE_UNKNOWN_OK);
                }
                if database {
                    siglevel.insert(SigLevel::DATABASE_MARGINAL_OK | SigLevel::DATABASE_UNKNOWN_OK);
                }
            }
            _ => return None,
        }
    }
    Some(siglevel)
}

impl Parser {
    pub fn with_default() -> Result<Self, ParseError> {
        Parser::with_file(PACMAN_CONF_PATH)
    }

    pub fn with_str(content: &impl ToString) -> Result<Self, ParseError> {
        let mut config = Ini::new();
        config
            .read(content.to_string())
            .map_err(ParseError::PacmanError)?;
        Ok(Self { config })
    }

    pub fn with_file(path: impl AsRef<str>) -> Result<Self, ParseError> {
        let mut config = Ini::new();
        config
            .load(path.as_ref())
            .map_err(ParseError::PacmanError)?;
        Ok(Self { config })
    }

    pub fn sync_dbs(&self) -> Vec<SyncDB> {
        let global_siglevel = self
            .config
            .get("options", "SigLevel")
            .and_then(|s| parse_siglevel(s.split(' ').collect::<Vec<&str>>()))
            .unwrap_or(SigLevel::USE_DEFAULT);

        self.config
            .get_map_ref()
            .iter()
            .filter_map(|(section, kv)| {
                if section == "options" {
                    None
                } else {
                    Some(SyncDB {
                        name: section.clone(),
                        sig_level: kv
                            .get("SigLevel")
                            .and_then(|v| {
                                v.as_ref()
                                    .and_then(|siglevel| parse_siglevel(siglevel.split(' ')))
                            })
                            .unwrap_or(global_siglevel),
                        server: kv.get("Server").and_then(Clone::clone),
                    })
                }
            })
            .collect::<Vec<_>>()
    }
}
