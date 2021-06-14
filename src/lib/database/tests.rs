use std::fs;
use std::io::Read;
use std::path::PathBuf;

use rstest::rstest;

use crate::database::pacman::{BuildTarget, DBBuilder};

use super::decompressor::ArchiveReader;

#[rstest]
#[case("test.tar")]
#[case("test.tar.gz")]
#[case("test.tar.xz")]
#[case("test.tar.zst")]
fn must_decompress(#[case] name: &str) {
    println!("decompressing {}", name);
    let path = PathBuf::from("tests/archives/").join(name);
    let archive = ArchiveReader::from_filepath(&path).expect("unable to read archive");
    let mut tar = archive.into_tar();
    let mut entries = tar.entries().expect("unable to read entries");

    let mut entry = entries
        .next()
        .expect("archive length mismatch")
        .expect("unable to read entry");
    assert_eq!(
        entry
            .path()
            .expect("unable to read path")
            .file_name()
            .unwrap()
            .to_str()
            .unwrap(),
        "test",
        "filename mismatch"
    );
    let mut buffer = String::new();
    entry
        .read_to_string(&mut buffer)
        .expect("unable to read file");
    assert_eq!(buffer, "test\n", "file content mismatch");
}

#[test]
pub fn must_build_dir() {
    let mut builder = DBBuilder::new();
    for path in fs::read_dir("tests/pkgs").expect("missing test directory") {
        let path = path.expect("invalid dir entry");
        builder.add_file_mut(path.path());
    }
    drop(fs::remove_dir_all("tests/output"));
    fs::create_dir("tests/output").expect("unable to create test directory");
    builder
        .build(BuildTarget::new("tests/output", None))
        .expect("unable to build db folder");
    builder
        .build(BuildTarget::new("tests/output", Some("test")))
        .expect("unable to build db archive");
}
