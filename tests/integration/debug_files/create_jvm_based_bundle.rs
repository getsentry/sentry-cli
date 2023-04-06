use crate::integration::{
    copy_recursively, mock_common_upload_endpoints, register_test, ServerBehavior,
};
use std::fs::{create_dir, remove_dir_all, write};

#[test]
fn command_create_jvm_based_bundle_help() {
    #[cfg(not(windows))]
    register_test("debug_files/debug_files-create-jvm-based-bundle-help.trycmd");
    #[cfg(windows)]
    register_test("debug_files/debug_files-create-jvm-based-bundle-help-windows.trycmd");
}

#[test]
fn command_create_jvm_based_bundle_fails_out_not_found() {
    let _upload_endpoints = mock_common_upload_endpoints(ServerBehavior::Legacy);
    #[cfg(not(windows))]
    register_test("debug_files/debug_files-create-jvm-based-bundle-output-not-found.trycmd");
    #[cfg(windows)]
    register_test("debug_files/debug_files-create-jvm-based-bundle-output-not-found-windows.trycmd");
}

#[test]
fn command_create_jvm_based_bundle_fails_out_is_file() {
    let testcase_cwd = "tests/integration/_cases/debug_files/debug_files-create-jvm-based-bundle-output-is-file.in/";
    let testcase_cwd_path = std::path::Path::new(testcase_cwd);
    if testcase_cwd_path.exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively("tests/integration/_fixtures/jvmbased/", testcase_cwd_path).unwrap();
    write(testcase_cwd_path.join("file.txt"), "some file content").unwrap();
    let _upload_endpoints = mock_common_upload_endpoints(ServerBehavior::Legacy);

    #[cfg(not(windows))]
    register_test("debug_files/debug_files-create-jvm-based-bundle-output-is-file.trycmd");
    #[cfg(windows)]
    register_test("debug_files/debug_files-create-jvm-based-bundle-output-is-file-windows.trycmd");
}

#[test]
fn command_create_jvm_based_bundle_fails_input_not_found() {
    let _upload_endpoints = mock_common_upload_endpoints(ServerBehavior::Legacy);
    register_test("debug_files/debug_files-create-jvm-based-bundle-input-not-found.trycmd");
}

#[test]
fn command_create_jvm_based_bundle_fails_input_is_file() {
    let _upload_endpoints = mock_common_upload_endpoints(ServerBehavior::Legacy);
    register_test("debug_files/debug_files-create-jvm-based-bundle-input-is-file.trycmd");
}

#[test]
fn command_create_jvm_based_bundle_input_dir_empty() {
    let testcase_cwd = "tests/integration/_cases/debug_files/debug_files-create-jvm-based-bundle-input-dir-empty.in/";
    let testcase_cwd_path = std::path::Path::new(testcase_cwd);
    if testcase_cwd_path.exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively("tests/integration/_fixtures/jvmbased/", testcase_cwd_path).unwrap();
    create_dir(testcase_cwd_path.join("empty-dir")).unwrap();
    let _upload_endpoints = mock_common_upload_endpoints(ServerBehavior::Legacy);
    register_test("debug_files/debug_files-create-jvm-based-bundle-input-dir-empty.trycmd");
}

#[test]
fn command_create_jvm_based_bundle_fails_invalid_uuid() {
    let _upload_endpoints = mock_common_upload_endpoints(ServerBehavior::Legacy);
    register_test("debug_files/debug_files-create-jvm-based-bundle-invalid-uuid.trycmd");
}

#[test]
fn command_create_jvm_based_bundle() {
    let testcase_cwd_path =
        "tests/integration/_cases/debug_files/debug_files-create-jvm-based-bundle.in/";
    if std::path::Path::new(testcase_cwd_path).exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively("tests/integration/_fixtures/jvmbased/", testcase_cwd_path).unwrap();
    let _upload_endpoints = mock_common_upload_endpoints(ServerBehavior::Legacy);
    register_test("debug_files/debug_files-create-jvm-based-bundle.trycmd");
}
