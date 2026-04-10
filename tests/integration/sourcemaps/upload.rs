use crate::integration::TestManager;

#[test]
fn command_sourcemaps_upload_help() {
    TestManager::new().register_trycmd_test("sourcemaps/sourcemaps-upload-help.trycmd");
}

#[test]
fn command_sourcemaps_upload_log_level_info() {
    TestManager::new()
        .mock_common_upload_endpoints(None, Some(vec!["95d152c0530efb498133138c7e7092612f5abab1"]))
        .register_trycmd_test("sourcemaps/sourcemaps-upload-log-level-info.trycmd")
        .with_default_token()
        .assert_mock_endpoints();
}

#[test]
fn command_sourcemaps_upload() {
    TestManager::new().register_trycmd_test("sourcemaps/sourcemaps-upload.trycmd");
}

#[test]
fn command_sourcemaps_upload_modern() {
    TestManager::new()
        .mock_common_upload_endpoints()
        .register_trycmd_test("sourcemaps/sourcemaps-upload-modern.trycmd")
        .with_default_token()
        .assert_mock_endpoints();
}

#[test]
fn command_sourcemaps_upload_modern_v2() {
    TestManager::new()
        .mock_common_upload_endpoints_with(Some(512), true)
        .register_trycmd_test("sourcemaps/sourcemaps-upload-modern.trycmd")
        .with_default_token()
        .assert_mock_endpoints();
}

#[test]
fn command_sourcemaps_upload_some_debugids() {
    TestManager::new()
        .mock_common_upload_endpoints()
        .register_trycmd_test("sourcemaps/sourcemaps-upload-some-debugids.trycmd")
        .with_default_token()
        .assert_mock_endpoints();
}

/// Tests that debug IDs can be found under the old "debug_id" field in sourcemaps.
#[test]
fn command_sourcemaps_upload_debugid_alias() {
    TestManager::new()
        .mock_common_upload_endpoints()
        .register_trycmd_test("sourcemaps/sourcemaps-upload-debugid-alias.trycmd")
        .with_default_token()
        .assert_mock_endpoints();
}

#[test]
fn command_sourcemaps_upload_no_debugids() {
    TestManager::new()
        .mock_common_upload_endpoints_with(None, false)
        .register_trycmd_test("sourcemaps/sourcemaps-upload-no-debugids.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_upload_file_ram_bundle() {
    TestManager::new()
        .mock_common_upload_endpoints()
        .register_trycmd_test("sourcemaps/sourcemaps-upload-file-ram-bundle.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_upload_indexed_ram_bundle() {
    TestManager::new()
        .mock_common_upload_endpoints()
        .register_trycmd_test("sourcemaps/sourcemaps-upload-indexed-ram-bundle.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_upload_hermes_bundle_with_referencing_debug_id() {
    TestManager::new()
        .mock_common_upload_endpoints()
        .register_trycmd_test(
            "sourcemaps/sourcemaps-upload-file-hermes-bundle-reference-debug-id.trycmd",
        )
        .with_default_token();
}

#[test]
fn command_sourcemaps_upload_cjs_mjs() {
    TestManager::new()
        .mock_common_upload_endpoints_with(None, false)
        .register_trycmd_test("sourcemaps/sourcemaps-upload-cjs-mjs.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_upload_complex_extension() {
    TestManager::new()
        .mock_common_upload_endpoints_with(None, false)
        .register_trycmd_test("sourcemaps/sourcemaps-upload-complex-extension.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_upload_skip_invalid_utf8() {
    TestManager::new()
        .mock_common_upload_endpoints_with(None, false)
        .register_trycmd_test("sourcemaps/sourcemaps-with-invalid-utf8.trycmd")
        .with_default_token();
}
