use crate::integration::TestManager;

#[test]
fn command_sourcemaps_upload_help() {
    TestManager::new().register_trycmd_test("sourcemaps/sourcemaps-upload-help.trycmd");
}

#[test]
fn command_sourcemaps_upload() {
    TestManager::new().register_trycmd_test("sourcemaps/sourcemaps-upload.trycmd");
}

#[test]
fn command_sourcemaps_upload_modern() {
    TestManager::new()
        .mock_common_upload_endpoints(None, Some(vec!["95d152c0530efb498133138c7e7092612f5abab1"]))
        .register_trycmd_test("sourcemaps/sourcemaps-upload-modern.trycmd")
        .with_default_token()
        .assert_mock_endpoints();
}

#[test]
fn command_sourcemaps_upload_modern_v2() {
    TestManager::new()
        .mock_common_upload_endpoints(
            Some(512),
            Some(vec!["ec8450a9db19805703a27a2545c18b7b27ba0d7d"]),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-upload-modern.trycmd")
        .with_default_token()
        .assert_mock_endpoints();
}

#[test]
fn command_sourcemaps_upload_some_debugids() {
    TestManager::new()
        .mock_common_upload_endpoints(None, Some(vec!["299cfc03739e780899877875d3c0681095ea91b7"]))
        .register_trycmd_test("sourcemaps/sourcemaps-upload-some-debugids.trycmd")
        .with_default_token();
}

/// Tests that debug IDs can be found under the "debugId" field in sourcemaps.
#[test]
fn command_sourcemaps_upload_debugid_alias() {
    TestManager::new()
        .mock_common_upload_endpoints(None, Some(vec!["63986f7f40fa1a55813a9106e8e5b63ce516246a"]))
        .register_trycmd_test("sourcemaps/sourcemaps-upload-debugid-alias.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_upload_no_debugids() {
    TestManager::new()
        .mock_common_upload_endpoints(None, None)
        .register_trycmd_test("sourcemaps/sourcemaps-upload-no-debugids.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_upload_file_ram_bundle() {
    TestManager::new()
        .mock_common_upload_endpoints(None, Some(vec!["e268173df7cbb38ca44334572c2815a264a2c28f"]))
        .register_trycmd_test("sourcemaps/sourcemaps-upload-file-ram-bundle.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_upload_indexed_ram_bundle() {
    TestManager::new()
        .mock_common_upload_endpoints(None, Some(vec!["857ff6e07491c487ae23fcacbee7359b45bdf390"]))
        .register_trycmd_test("sourcemaps/sourcemaps-upload-indexed-ram-bundle.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_upload_hermes_bundle_with_referencing_debug_id() {
    TestManager::new()
        .mock_common_upload_endpoints(None, Some(vec!["cdbd84e597e6a28c88924e82414fef60a7f3ff86"]))
        .register_trycmd_test(
            "sourcemaps/sourcemaps-upload-file-hermes-bundle-reference-debug-id.trycmd",
        )
        .with_default_token();
}

#[test]
fn command_sourcemaps_upload_cjs_mjs() {
    TestManager::new()
        .mock_common_upload_endpoints(None, None)
        .register_trycmd_test("sourcemaps/sourcemaps-upload-cjs-mjs.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_upload_complex_extension() {
    TestManager::new()
        .mock_common_upload_endpoints(None, None)
        .register_trycmd_test("sourcemaps/sourcemaps-upload-complex-extension.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_upload_skip_invalid_utf8() {
    TestManager::new()
        .mock_common_upload_endpoints(None, None)
        .register_trycmd_test("sourcemaps/sourcemaps-with-invalid-utf8.trycmd")
        .with_default_token();
}
