use std::path::{Path, PathBuf};

use alpm::SigLevel;
use ini::Ini;

use crate::error::ParseError;

type Result<T> = std::result::Result<T, ParseError>;

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

pub struct PacmanParser {
    config: Ini,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SyncDB {
    pub name: String,
    pub sig_level: alpm::SigLevel,
    pub servers: Vec<String>,
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

impl PacmanParser {
    pub fn with_default() -> Result<Self> {
        Self::with_pacman_conf(&PacmanConfCtx::default())
    }

    pub fn with_pacman_conf(ctx: &PacmanConfCtx) -> Result<Self> {
        let mut cmd = std::process::Command::new("pacman-conf");
        if let Some(path) = &ctx.path {
            let canonical_path = path.canonicalize()?;
            cmd.current_dir(canonical_path.parent().unwrap());
            cmd.arg("-c").arg(canonical_path.file_name().unwrap());
        }
        if let Some(root) = &ctx.root {
            cmd.arg("-R").arg(root);
        }

        let raw_conf = cmd.output()?;

        Self::with_str(
            std::str::from_utf8(&*raw_conf.stdout)
                .map_err(|_| ParseError::PacmanError(String::from("utf8 parse error")))?,
        )
    }

    pub fn with_str(content: impl AsRef<str>) -> Result<Self> {
        Ok(Self {
            config: Ini::load_from_str(content.as_ref())
                .map_err(|e| ParseError::PacmanError(e.to_string()))?,
        })
    }

    pub fn sync_dbs(&self) -> Vec<SyncDB> {
        let global_siglevel = self
            .config
            .section(Some("options"))
            .map(|options| {
                options
                    .get_all("SigLevel")
                    .flat_map(|field| field.split(' '))
            })
            .and_then(parse_siglevel)
            .unwrap_or(SigLevel::USE_DEFAULT);

        self.config
            .sections()
            .filter(|section| section.map(|name| name != "options").unwrap_or(false))
            .map(|name| (name.unwrap(), self.config.section(name).unwrap()))
            .map(|(name, section)| SyncDB {
                name: name.to_string(),
                sig_level: parse_siglevel(
                    section
                        .get_all("SigLevel")
                        .flat_map(|field| field.split(' ')),
                )
                .unwrap_or(global_siglevel),
                servers: section.get_all("Server").map(ToString::to_string).collect(),
            })
            .collect()
    }
}
