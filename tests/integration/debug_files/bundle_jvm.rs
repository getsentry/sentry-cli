use crate::integration::{copy_recursively, TestManager};
use std::fs::{create_dir, remove_dir_all, write};

#[test]
fn command_bundle_jvm_out_not_found_creates_dir() {
    let testcase_cwd =
        "tests/integration/_cases/debug_files/bundle_jvm/debug_files-bundle-jvm-output-not-found.in/";
    let testcase_cwd_path = std::path::Path::new(testcase_cwd);
    if testcase_cwd_path.exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively(
        "tests/integration/_fixtures/jvm",
        testcase_cwd_path.join("jvm"),
    )
    .unwrap();

    TestManager::new()
        .register_trycmd_test(
            "debug_files/bundle_jvm/debug_files-bundle-jvm-output-not-found.trycmd",
        )
        .with_default_token();
}

#[test]
fn command_bundle_jvm_fails_out_is_file() {
    let testcase_cwd =
        "tests/integration/_cases/debug_files/bundle_jvm/debug_files-bundle-jvm-output-is-file.in/";
    let testcase_cwd_path = std::path::Path::new(testcase_cwd);
    if testcase_cwd_path.exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively("tests/integration/_fixtures/jvm/", testcase_cwd_path).unwrap();
    write(testcase_cwd_path.join("file.txt"), "some file content").unwrap();

    TestManager::new()
        .register_trycmd_test("debug_files/bundle_jvm/debug_files-bundle-jvm-output-is-file.trycmd")
        .with_default_token();
}

#[test]
fn command_bundle_jvm_fails_input_not_found() {
    TestManager::new()
        .register_trycmd_test(
            "debug_files/bundle_jvm/debug_files-bundle-jvm-input-not-found.trycmd",
        )
        .with_default_token();
}

#[test]
fn command_bundle_jvm_fails_input_is_file() {
    TestManager::new()
        .register_trycmd_test("debug_files/bundle_jvm/debug_files-bundle-jvm-input-is-file.trycmd")
        .with_default_token();
}

#[test]
fn command_bundle_jvm_input_dir_empty() {
    let testcase_cwd =
        "tests/integration/_cases/debug_files/bundle_jvm/debug_files-bundle-jvm-input-dir-empty.in/";
    let testcase_cwd_path = std::path::Path::new(testcase_cwd);
    if testcase_cwd_path.exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively("tests/integration/_fixtures/jvm/", testcase_cwd_path).unwrap();
    create_dir(testcase_cwd_path.join("empty-dir")).unwrap();
    TestManager::new()
        .register_trycmd_test(
            "debug_files/bundle_jvm/debug_files-bundle-jvm-input-dir-empty.trycmd",
        )
        .with_default_token();
}

#[test]
fn command_bundle_jvm_fails_invalid_uuid() {
    TestManager::new()
        .register_trycmd_test("debug_files/bundle_jvm/debug_files-bundle-jvm-invalid-uuid.trycmd");
}

#[test]
fn command_bundle_jvm() {
    let testcase_cwd_path =
        "tests/integration/_cases/debug_files/bundle_jvm/debug_files-bundle-jvm.in/";
    if std::path::Path::new(testcase_cwd_path).exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively("tests/integration/_fixtures/jvm/", testcase_cwd_path).unwrap();
    TestManager::new()
        .register_trycmd_test("debug_files/bundle_jvm/debug_files-bundle-jvm.trycmd")
        .with_default_token();
}
