use std::fs::remove_dir_all;

use crate::integration::{copy_recursively, register_test};

#[test]
fn command_sourcemaps_inject_help() {
    register_test("sourcemaps/sourcemaps-inject-help.trycmd");
}

#[test]
fn command_sourcemaps_inject_output() {
    remove_dir_all("tests/integration/_cases/sourcemaps/sourcemaps-inject.in/").unwrap();
    copy_recursively(
        "tests/integration/_fixtures/inject/",
        "tests/integration/_cases/sourcemaps/sourcemaps-inject.in/",
    )
    .unwrap();

    register_test("sourcemaps/sourcemaps-inject.trycmd");
}
