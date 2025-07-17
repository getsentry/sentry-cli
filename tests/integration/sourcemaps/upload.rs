use crate::integration::{ChunkOptions, MockEndpointBuilder, ServerBehavior, TestManager};

#[test]
fn command_sourcemaps_upload_help() {
    TestManager::new().register_trycmd_test("sourcemaps/sourcemaps-upload-help.trycmd");
}

#[test]
fn command_sourcemaps_upload() {
    TestManager::new().register_trycmd_test("sourcemaps/sourcemaps-upload.trycmd");
}

#[test]
fn command_sourcemaps_upload_successfully_upload_file() {
    TestManager::new()
        .mock_common_upload_endpoints(ServerBehavior::Legacy, Default::default())
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/releases/wat-release/files/?cursor=&checksum=38ed853073df85147960ea3a5bced6170ec389b0",
            )
            .with_response_body("[]"),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-upload-successfully-upload-file.trycmd")
        .with_default_token()
        .assert_mock_endpoints();
}

#[test]
fn command_sourcemaps_upload_skip_already_uploaded() {
    TestManager::new()
        .mock_common_upload_endpoints(ServerBehavior::Legacy, Default::default())
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/releases/wat-release/files/?cursor=&checksum=38ed853073df85147960ea3a5bced6170ec389b0&checksum=f3673e2cea68bcb86bb74254a9efaa381d74929f",
            )
            .with_response_body(
                r#"[{
                    "id": "1337",
                    "name": "~/vendor.min.js.map",
                    "headers": {},
                    "size": 1522,
                    "sha1": "f3673e2cea68bcb86bb74254a9efaa381d74929f",
                    "dateCreated": "2022-05-12T11:08:01.496220Z"
                }]"#,
            ),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-upload-skip-already-uploaded.trycmd")
        .with_default_token()
        .assert_mock_endpoints();
}

#[test]
fn command_sourcemaps_upload_no_dedupe() {
    TestManager::new()
        .mock_common_upload_endpoints(ServerBehavior::Legacy, Default::default())
        .register_trycmd_test("sourcemaps/sourcemaps-upload-no-dedupe.trycmd")
        .with_default_token()
        .assert_mock_endpoints();
}

#[test]
fn command_sourcemaps_upload_modern() {
    TestManager::new()
        .mock_common_upload_endpoints(ServerBehavior::Modern, Default::default())
        .register_trycmd_test("sourcemaps/sourcemaps-upload-modern.trycmd")
        .with_default_token()
        .assert_mock_endpoints();
}

#[test]
fn command_sourcemaps_upload_modern_v2() {
    TestManager::new()
        .mock_common_upload_endpoints(
            ServerBehavior::ModernV2,
            ChunkOptions {
                missing_chunks: vec!["ec8450a9db19805703a27a2545c18b7b27ba0d7d".to_owned()],
                // Set the chunk size so the bundle will be split into two chunks
                chunk_size: 512,
            },
        )
        .register_trycmd_test("sourcemaps/sourcemaps-upload-modern.trycmd")
        .with_default_token()
        .assert_mock_endpoints();
}

#[test]
fn command_sourcemaps_upload_empty() {
    TestManager::new()
        .mock_common_upload_endpoints(ServerBehavior::Legacy, Default::default())
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/releases/wat-release/files/?cursor=",
            )
            .with_response_body("[]"),
        )
        .register_trycmd_test("releases/releases-files-upload-sourcemaps.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_upload_some_debugids() {
    TestManager::new()
        .mock_common_upload_endpoints(ServerBehavior::Modern, Default::default())
        .register_trycmd_test("sourcemaps/sourcemaps-upload-some-debugids.trycmd")
        .with_default_token()
        .assert_mock_endpoints();
}

#[test]
fn command_sourcemaps_upload_some_debugids_v2() {
    TestManager::new()
        .mock_common_upload_endpoints(
            ServerBehavior::ModernV2,
            ChunkOptions {
                missing_chunks: vec!["ff16e0ac593a74b454cc34814f6249f45a1a2dfe".to_owned()],
                chunk_size: 524288,
            },
        )
        .register_trycmd_test("sourcemaps/sourcemaps-upload-some-debugids.trycmd")
        .with_default_token()
        .assert_mock_endpoints();
}

/// Tests that debug IDs can be found under the "debugId" field in sourcemaps.
#[test]
fn command_sourcemaps_upload_debugid_alias() {
    TestManager::new()
        .mock_common_upload_endpoints(ServerBehavior::Modern, Default::default())
        .register_trycmd_test("sourcemaps/sourcemaps-upload-debugid-alias.trycmd")
        .with_default_token()
        .assert_mock_endpoints();
}

#[test]
fn command_sourcemaps_upload_no_debugids() {
    TestManager::new()
        .mock_common_upload_endpoints(ServerBehavior::Modern, Default::default())
        .register_trycmd_test("sourcemaps/sourcemaps-upload-no-debugids.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_upload_file_ram_bundle() {
    TestManager::new()
        .mock_common_upload_endpoints(ServerBehavior::Modern, Default::default())
        .register_trycmd_test("sourcemaps/sourcemaps-upload-file-ram-bundle.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_upload_indexed_ram_bundle() {
    TestManager::new()
        .mock_common_upload_endpoints(ServerBehavior::Modern, Default::default())
        .register_trycmd_test("sourcemaps/sourcemaps-upload-indexed-ram-bundle.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_upload_hermes_bundle_with_referencing_debug_id() {
    TestManager::new()
        .mock_common_upload_endpoints(ServerBehavior::Modern, Default::default())
        .register_trycmd_test(
            "sourcemaps/sourcemaps-upload-file-hermes-bundle-reference-debug-id.trycmd",
        )
        .with_default_token();
}

#[test]
fn command_sourcemaps_upload_cjs_mjs() {
    TestManager::new()
        .mock_common_upload_endpoints(ServerBehavior::Modern, Default::default())
        .register_trycmd_test("sourcemaps/sourcemaps-upload-cjs-mjs.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_upload_complex_extension() {
    TestManager::new()
        .mock_common_upload_endpoints(ServerBehavior::Modern, Default::default())
        .register_trycmd_test("sourcemaps/sourcemaps-upload-complex-extension.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_upload_skip_invalid_utf8() {
    TestManager::new()
        .mock_common_upload_endpoints(ServerBehavior::Modern, Default::default())
        .register_trycmd_test("sourcemaps/sourcemaps-with-invalid-utf8.trycmd")
        .with_default_token();
}
