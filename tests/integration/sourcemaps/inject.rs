use std::fs::remove_dir_all;

use crate::integration::{copy_recursively, register_test};

#[test]
fn command_sourcemaps_inject_help() {
    register_test("sourcemaps/sourcemaps-inject-help.trycmd");
}

#[test]
fn command_sourcemaps_inject_output() {
    let testcase_cwd_path = "tests/integration/_cases/sourcemaps/sourcemaps-inject.in/";
    if std::path::Path::new(testcase_cwd_path).exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively("tests/integration/_fixtures/inject/", testcase_cwd_path).unwrap();

    register_test("sourcemaps/sourcemaps-inject.trycmd");
}

#[test]
fn command_sourcemaps_inject_output_nomappings() {
    let testcase_cwd_path = "tests/integration/_cases/sourcemaps/sourcemaps-inject-nomappings.in/";
    if std::path::Path::new(testcase_cwd_path).exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively(
        "tests/integration/_fixtures/inject_nomappings/",
        testcase_cwd_path,
    )
    .unwrap();

    register_test("sourcemaps/sourcemaps-inject-nomappings.trycmd");
}
