use alpm::SigLevel;

use crate::parser::{PacmanConf, PacmanConfCtx, SyncDB};

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

    let expect_mirrors = vec![
        "http://mirrors.evowise.com/archlinux/$repo/os/$arch",
        "http://mirror.rackspace.com/archlinux/$repo/os/$arch",
        "https://mirror.rackspace.com/archlinux/$repo/os/$arch",
    ];
    let expect_mirror_list = include_str!("../../../tests/pacman_conf/mirrorlist_clean");
    let expect_sync_dbs = vec![
        SyncDB {
            name: String::from("core"),
            sig_level,
            servers: servers("core"),
            usage: vec![String::from("All")],
        },
        SyncDB {
            name: String::from("extra"),
            sig_level,
            servers: servers("extra"),
            usage: vec![String::from("All")],
        },
        SyncDB {
            name: String::from("community"),
            sig_level,
            servers: servers("community"),
            usage: vec![String::from("All")],
        },
        SyncDB {
            name: String::from("archlinuxcn"),
            sig_level: SigLevel::PACKAGE | SigLevel::DATABASE | SigLevel::DATABASE_OPTIONAL,
            servers: vec![String::from(
                "https://mirror.sjtu.edu.cn/archlinux-cn/x86_64",
            )],
            usage: vec![String::from("All")],
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
            usage: vec![String::from("All")],
        },
    ];

    let parser = PacmanConf::with(&PacmanConfCtx::new().path("tests/pacman_conf/pacman.conf"))
        .expect("unable to parse config");
    let dbs = parser.sync_dbs();
    assert_eq!(dbs, &expect_sync_dbs, "sync dbs mismatch");

    let mirrors = parser.host_mirrors();
    let mirror_list = parser.mirror_list();
    assert_eq!(mirrors, expect_mirrors, "mirrors mismatch");
    assert_eq!(mirror_list, expect_mirror_list, "mirror list mismatch");

    assert_eq!(parser.option("nonsense"), None);
    assert_eq!(parser.option("GPGDir"), Some("/etc/pacman.d/gnupg/"));
    assert_eq!(parser.option("LogFile"), Some("/var/log/pacman_archer.log"));
    assert_eq!(parser.option("RootDir"), Some("/"));

    let parser_with_root = PacmanConf::with(
        &PacmanConfCtx::new()
            .path("tests/pacman_conf/pacman.conf")
            .root("/archer"),
    )
    .expect("unable to parse config");
    assert_eq!(parser_with_root.option("RootDir"), Some("/archer"));
}
