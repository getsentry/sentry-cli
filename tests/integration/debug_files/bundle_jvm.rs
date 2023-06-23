use crate::integration::{
    copy_recursively, mock_common_upload_endpoints, register_test, ServerBehavior,
};
use std::fs::{create_dir, remove_dir_all, write};

#[test]
fn command_bundle_jvm_help() {
    register_test("debug_files/debug_files-bundle-jvm-help.trycmd");
}

#[test]
fn command_bundle_jvm_out_not_found_creates_dir() {
    let testcase_cwd =
        "tests/integration/_cases/debug_files/debug_files-bundle-jvm-output-not-found.in/";
    let testcase_cwd_path = std::path::Path::new(testcase_cwd);
    if testcase_cwd_path.exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively(
        "tests/integration/_fixtures/jvm",
        testcase_cwd_path.join("jvm"),
    )
    .unwrap();
    let _upload_endpoints =
        mock_common_upload_endpoints(ServerBehavior::Legacy, Default::default());
    register_test("debug_files/debug_files-bundle-jvm-output-not-found.trycmd");
}

#[test]
fn command_bundle_jvm_fails_out_is_file() {
    let testcase_cwd =
        "tests/integration/_cases/debug_files/debug_files-bundle-jvm-output-is-file.in/";
    let testcase_cwd_path = std::path::Path::new(testcase_cwd);
    if testcase_cwd_path.exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively("tests/integration/_fixtures/jvm/", testcase_cwd_path).unwrap();
    write(testcase_cwd_path.join("file.txt"), "some file content").unwrap();
    let _upload_endpoints =
        mock_common_upload_endpoints(ServerBehavior::Legacy, Default::default());

    register_test("debug_files/debug_files-bundle-jvm-output-is-file.trycmd");
}

#[test]
fn command_bundle_jvm_fails_input_not_found() {
    let _upload_endpoints =
        mock_common_upload_endpoints(ServerBehavior::Legacy, Default::default());
    register_test("debug_files/debug_files-bundle-jvm-input-not-found.trycmd");
}

#[test]
fn command_bundle_jvm_fails_input_is_file() {
    let _upload_endpoints =
        mock_common_upload_endpoints(ServerBehavior::Legacy, Default::default());
    register_test("debug_files/debug_files-bundle-jvm-input-is-file.trycmd");
}

#[test]
fn command_bundle_jvm_input_dir_empty() {
    let testcase_cwd =
        "tests/integration/_cases/debug_files/debug_files-bundle-jvm-input-dir-empty.in/";
    let testcase_cwd_path = std::path::Path::new(testcase_cwd);
    if testcase_cwd_path.exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively("tests/integration/_fixtures/jvm/", testcase_cwd_path).unwrap();
    create_dir(testcase_cwd_path.join("empty-dir")).unwrap();
    let _upload_endpoints =
        mock_common_upload_endpoints(ServerBehavior::Legacy, Default::default());
    register_test("debug_files/debug_files-bundle-jvm-input-dir-empty.trycmd");
}

#[test]
fn command_bundle_jvm_fails_invalid_uuid() {
    let _upload_endpoints =
        mock_common_upload_endpoints(ServerBehavior::Legacy, Default::default());
    register_test("debug_files/debug_files-bundle-jvm-invalid-uuid.trycmd");
}

#[test]
fn command_bundle_jvm() {
    let testcase_cwd_path = "tests/integration/_cases/debug_files/debug_files-bundle-jvm.in/";
    if std::path::Path::new(testcase_cwd_path).exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively("tests/integration/_fixtures/jvm/", testcase_cwd_path).unwrap();
    let _upload_endpoints =
        mock_common_upload_endpoints(ServerBehavior::Legacy, Default::default());
    register_test("debug_files/debug_files-bundle-jvm.trycmd");
}
