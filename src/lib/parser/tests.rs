use alpm::SigLevel;

use crate::parser::{PacmanConfCtx, PacmanParser, SyncDB};

#[test]
fn must_parse_pacman() {
    let sig_level = SigLevel::PACKAGE | SigLevel::DATABASE | SigLevel::DATABASE_OPTIONAL;
    let servers = |repo: &str| -> Vec<String> {
        vec![
            format!("http://mirrors.evowise.com/archlinux/{}/os/x86_64", repo),
            format!("http://mirror.rackspace.com/archlinux/{}/os/x86_64", repo),
            format!("https://mirror.rackspace.com/archlinux/{}/os/x86_64", repo),
        ]
    };
    let expect = vec![
        SyncDB {
            name: String::from("core"),
            sig_level,
            servers: servers("core"),
        },
        SyncDB {
            name: String::from("extra"),
            sig_level,
            servers: servers("extra"),
        },
        SyncDB {
            name: String::from("community"),
            sig_level,
            servers: servers("community"),
        },
        SyncDB {
            name: String::from("archlinuxcn"),
            sig_level: SigLevel::PACKAGE | SigLevel::DATABASE | SigLevel::DATABASE_OPTIONAL,
            servers: vec![String::from(
                "https://mirror.sjtu.edu.cn/archlinux-cn/x86_64",
            )],
        },
        SyncDB {
            name: String::from("custom"),
            sig_level: SigLevel::PACKAGE
                | SigLevel::PACKAGE_OPTIONAL
                | SigLevel::DATABASE
                | SigLevel::DATABASE_OPTIONAL
                | SigLevel::PACKAGE_MARGINAL_OK
                | SigLevel::PACKAGE_UNKNOWN_OK
                | SigLevel::DATABASE_MARGINAL_OK
                | SigLevel::DATABASE_UNKNOWN_OK,
            servers: vec![String::from("file:///home/custompkgs")],
        },
    ];

    let parser =
        PacmanParser::with_pacman_conf(&PacmanConfCtx::new().path("tests/pacman_conf/pacman.conf"))
            .expect("unable to parse config");
    let dbs = parser.sync_dbs();
    assert_eq!(dbs, expect, "sync dbs mismatch");
}
