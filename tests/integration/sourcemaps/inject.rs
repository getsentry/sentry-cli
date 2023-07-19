use std::fs::{self, remove_dir_all};

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

#[test]
fn command_sourcemaps_inject_output_nofiles() {
    let testcase_cwd_path = "tests/integration/_cases/sourcemaps/sourcemaps-inject-nofiles.in/";
    if std::path::Path::new(testcase_cwd_path).exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    fs::create_dir_all(std::path::Path::new(testcase_cwd_path).join("nonexisting")).unwrap();

    register_test("sourcemaps/sourcemaps-inject-nofiles.trycmd");
}

#[test]
fn command_sourcemaps_inject_output_embedded() {
    let testcase_cwd_path =
        std::path::Path::new("tests/integration/_cases/sourcemaps/sourcemaps-inject-embedded.in/");
    if testcase_cwd_path.exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    fs::create_dir_all(testcase_cwd_path).unwrap();
    fs::copy(
        "tests/integration/_fixtures/inject/server/dummy_embedded.js",
        testcase_cwd_path.join("dummy_embedded.js"),
    )
    .unwrap();

    // Check mappings before injection
    let contents = std::fs::read_to_string(testcase_cwd_path.join("dummy_embedded.js")).unwrap();
    let encoded_sourcemap = contents
        .lines()
        .find_map(|line| line.strip_prefix("//# sourceMappingURL=data:application/json;base64,"))
        .unwrap();

    let decoded = data_encoding::BASE64
        .decode(encoded_sourcemap.as_bytes())
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&decoded).unwrap();
    assert_eq!(parsed["mappings"], ";;;");

    register_test("sourcemaps/sourcemaps-inject-embedded.trycmd");

    // Check mappings after injection
    let contents = std::fs::read_to_string(testcase_cwd_path.join("dummy_embedded.js")).unwrap();
    let encoded_sourcemap = contents
        .lines()
        .find_map(|line| line.strip_prefix("//# sourceMappingURL=data:application/json;base64,"))
        .unwrap();

    let decoded = data_encoding::BASE64
        .decode(encoded_sourcemap.as_bytes())
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&decoded).unwrap();
    assert_eq!(parsed["mappings"], ";;;;");
}

#[test]
fn command_sourcemaps_inject_output_split() {
    let testcase_cwd_path = "tests/integration/_cases/sourcemaps/sourcemaps-inject-split.in/";
    if std::path::Path::new(testcase_cwd_path).exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively(
        "tests/integration/_fixtures/inject_split/",
        testcase_cwd_path,
    )
    .unwrap();

    register_test("sourcemaps/sourcemaps-inject-split.trycmd");
}

#[test]
fn command_sourcemaps_inject_output_split_ambiguous() {
    let testcase_cwd_path =
        "tests/integration/_cases/sourcemaps/sourcemaps-inject-split-ambiguous.in/";
    if std::path::Path::new(testcase_cwd_path).exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively(
        "tests/integration/_fixtures/inject_split_ambiguous/",
        testcase_cwd_path,
    )
    .unwrap();

    register_test("sourcemaps/sourcemaps-inject-split-ambiguous.trycmd");
}
