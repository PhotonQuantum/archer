use std::path::{Path, PathBuf};

use alpm::SigLevel;
use ini::Ini;
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::{NoExpand, Regex};

use crate::consts::PACMAN_CONF_PATH;
use crate::error::ParseError;

type Result<T> = std::result::Result<T, ParseError>;

lazy_static! {
    static ref RE_EXTRA: Regex = Regex::new(r"extra/os/.*").unwrap();
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Hash)]
pub struct PacmanConfCtx {
    path: Option<PathBuf>,
    root: Option<PathBuf>,
}

impl PacmanConfCtx {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn path(self, path: impl AsRef<Path>) -> Self {
        Self {
            path: Some(path.as_ref().to_path_buf()),
            ..self
        }
    }

    pub fn root(self, root: impl AsRef<Path>) -> Self {
        Self {
            root: Some(root.as_ref().to_path_buf()),
            ..self
        }
    }
}

#[derive(Clone)]
pub struct PacmanConf {
    inner: Ini,
    sync_dbs: Vec<SyncDB>,
    path: PathBuf,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SyncDB {
    pub name: String,
    pub sig_level: alpm::SigLevel,
    pub servers: Vec<String>,
    pub usage: Vec<String>,
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
    if siglevel == SigLevel::NONE {
        None
    } else {
        Some(siglevel)
    }
}

impl PacmanConf {
    pub fn new() -> Result<Self> {
        Self::with(&PacmanConfCtx::default())
    }

    pub fn with(ctx: &PacmanConfCtx) -> Result<Self> {
        let mut cmd = std::process::Command::new("pacman-conf");
        if let Some(path) = &ctx.path {
            let canonical_path = path.canonicalize()?;
            cmd.current_dir(canonical_path.parent().unwrap());
            cmd.arg("-c").arg(canonical_path.file_name().unwrap());
        }
        if let Some(root) = &ctx.root {
            cmd.arg("-R").arg(root);
        }

        let output = cmd.output()?;
        let raw_conf = std::str::from_utf8(&*output.stdout)
            .map_err(|_| ParseError::PacmanError(String::from("utf8 parse error")))?;

        let ini =
            Ini::load_from_str(raw_conf).map_err(|e| ParseError::PacmanError(e.to_string()))?;
        let sync_dbs = Self::parse_sync_dbs(&ini);

        let path = ctx
            .path
            .as_ref()
            .map_or_else(|| PathBuf::from(PACMAN_CONF_PATH), |path| path.clone());

        Ok(Self {
            inner: ini,
            sync_dbs,
            path,
        })
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    pub fn option(&self, field: &str) -> Option<&str> {
        self.inner
            .section(Some("options"))
            .and_then(|options| options.get(field))
    }

    pub fn host_mirrors(&self) -> Vec<String> {
        self.sync_dbs()
            .iter()
            .find(|db| db.name == "extra")
            .unwrap()
            .servers
            .iter()
            .map(|server| {
                RE_EXTRA
                    .replace(server, NoExpand("$repo/os/$arch"))
                    .to_string()
            })
            .collect()
    }

    pub fn mirror_list(&self) -> String {
        self.host_mirrors()
            .into_iter()
            .map(|server| format!("Server = {}", server))
            .join("\n")
    }

    pub fn sync_dbs(&self) -> &[SyncDB] {
        &self.sync_dbs
    }

    fn parse_sync_dbs(ini: &Ini) -> Vec<SyncDB> {
        let global_siglevel = ini
            .section(Some("options"))
            .map(|options| options.get_all("SigLevel"))
            .and_then(parse_siglevel)
            .unwrap_or(SigLevel::USE_DEFAULT);

        ini.sections()
            .filter(|section| section.map(|name| name != "options").unwrap_or(false))
            .map(|name| (name.unwrap(), ini.section(name).unwrap()))
            .map(|(name, section)| SyncDB {
                name: name.to_string(),
                sig_level: parse_siglevel(section.get_all("SigLevel")).unwrap_or(global_siglevel),
                servers: section.get_all("Server").map(ToString::to_string).collect(),
                usage: section.get_all("Usage").map(ToString::to_string).collect(),
            })
            .collect()
    }
}
