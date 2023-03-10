use crate::integration::register_test;
use assert_fs::prelude::*;

#[test]
fn command_sourcemaps_inject_help() {
    register_test("sourcemaps/sourcemaps-inject-help.trycmd");
}

#[test]
fn command_sourcemaps_inject_output() {
    let temp = assert_fs::TempDir::new().unwrap();
    let fixtures_path = temp.as_os_str().to_str().unwrap().to_string();
    temp.copy_from("tests/integration/_fixtures/inject", &["*"])
        .unwrap();

    let t = register_test("sourcemaps/sourcemaps-inject.trycmd");
    t.env("FIXTURES_PATH", fixtures_path);
}
