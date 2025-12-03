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
        .mock_common_upload_endpoints(None, Some(vec!["5c854c641249fb5ba1075735c68980f9f7ed72b6"]))
        .register_trycmd_test("sourcemaps/sourcemaps-upload-some-debugids.trycmd")
        .with_default_token()
        .assert_mock_endpoints();
}

/// Tests that debug IDs can be found under the old "debug_id" field in sourcemaps.
#[test]
fn command_sourcemaps_upload_debugid_alias() {
    TestManager::new()
        .mock_common_upload_endpoints(None, Some(vec!["4d668e3d5e4e436057d4b7157a450a9b7f130dfa"]))
        .register_trycmd_test("sourcemaps/sourcemaps-upload-debugid-alias.trycmd")
        .with_default_token()
        .assert_mock_endpoints();
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
        .mock_common_upload_endpoints(None, Some(vec!["47ef8e33f7213b9baa452715d04e251c090d0aaa"]))
        .register_trycmd_test("sourcemaps/sourcemaps-upload-indexed-ram-bundle.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_upload_hermes_bundle_with_referencing_debug_id() {
    TestManager::new()
        .mock_common_upload_endpoints(None, Some(vec!["a67a1e76159fc49e2fcc432fba8dbcd5d9696a73"]))
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
