use fs_extra;
use fs_extra::dir::CopyOptions;
use tempfile::tempdir;

use crate::builder::{BareBuildOptions, BareBuilder, BuildOptions, Builder};
use crate::tests::*;

fn setup_bare_builder() -> BareBuilder {
    let options = BareBuildOptions::new(&BuildOptions::new());
    let options = if let Some(user) = option_env!("BUILD_USER") {
        options.build_as(user)
    } else {
        options
    };
    BareBuilder::new_with_options(&options)
}

async fn build_install_a(builder: &impl Builder) {
    builder
        .install_remote(&["gcc", "make"])
        .await
        .expect("unable to install remote package");
    let workdir = tempdir().expect("unable to create workdir");
    fs_extra::dir::copy(
        "tests/build/archer_dummy_a",
        workdir.path(),
        &CopyOptions::new(),
    )
    .expect("unable to copy files");
    let mut files = builder
        .build(&*workdir.path().join("archer_dummy_a"))
        .await
        .expect("unable to build package");
    assert_eq!(files.len(), 1, "package count mismatch");
    let file = files.pop().unwrap();
    assert!(
        file.file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("archer_dummy_a"),
        "package name mismatch"
    );
    builder
        .install_local(&file)
        .await
        .expect("unable to install local package");
}

async fn build_install_b(builder: &impl Builder) {
    let workdir = tempdir().expect("unable to create workdir");
    fs_extra::dir::copy(
        "tests/build/archer_dummy_b",
        workdir.path(),
        &CopyOptions::new(),
    )
    .expect("unable to copy files");
    let files = builder
        .build(&workdir.path().join("archer_dummy_b"))
        .await
        .expect("unable to build package");
    assert_eq!(files.len(), 2, "package count mismatch");
    let mut marks = [false; 2];
    for file in &files {
        let filename = file.file_name().unwrap().to_str().unwrap();
        if filename.contains("archer_dummy_b_1") {
            marks[0] = true;
        }
        if filename.contains("archer_dummy_b_2") {
            marks[1] = true;
        }
    }
    assert!(marks.iter().all(|i| *i), "package name mismatch");

    // join tasks to test pacman mutex
    let install_tasks = files.iter().map(|file| builder.install_local(file));
    futures::future::join_all(install_tasks)
        .await
        .into_iter()
        .all(|result| result.is_ok())
        .then_some(())
        .expect("unable to install multiple packages");
}

fn must_a() {
    let output = std::process::Command::new("archer_dummy_a")
        .output()
        .expect("missing binary");
    assert_eq!(
        std::str::from_utf8(&output.stdout).expect("unable to parse output"),
        "dummy a\n",
        "output mismatch"
    );
}

fn must_b() {
    let output = std::process::Command::new("archer_dummy_b_1")
        .output()
        .expect("missing binary");
    assert_eq!(
        std::str::from_utf8(&output.stdout).expect("unable to parse output"),
        "dummy b 1\n",
        "output mismatch"
    );
    let output = std::process::Command::new("archer_dummy_b_2")
        .output()
        .expect("missing binary");
    assert_eq!(
        std::str::from_utf8(&output.stdout).expect("unable to parse output"),
        "dummy b 2\n",
        "output mismatch"
    );
}

async fn bare_cleanup(builder: &BareBuilder) {
    builder
        .remove(&["archer_dummy_a", "archer_dummy_b_1", "archer_dummy_b_2"])
        .await
        .expect("unable to uninstall packages")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn must_bare_build() {
    if option_env!("no_sudo").is_some() {
        println!("must_bare_build skipped");
        return;
    }
    wait_pacman_lock();
    let builder = setup_bare_builder();
    build_install_a(&builder).await;
    must_a();
    build_install_b(&builder).await;
    must_b();
    bare_cleanup(&builder).await;
}
