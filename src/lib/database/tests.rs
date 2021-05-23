use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::str::FromStr;

use itertools::Itertools;
use rstest::rstest;

use super::decompressor::Archive;

#[rstest]
#[case("test.tar")]
#[case("test.tar.gz")]
#[case("test.tar.xz")]
#[case("test.tar.zst")]
fn must_decompress(#[case] name: &str) {
    println!("decompressing {}", name);
    let path = PathBuf::from_str("tests/").unwrap().join(name);
    let archive = Archive::from_file(&path).expect("unable to read archive");
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
