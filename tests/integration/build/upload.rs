use std::sync::atomic::{AtomicBool, Ordering};

use crate::integration::test_utils::chunk_upload;
use crate::integration::{AssertCommand, MockEndpointBuilder, TestManager};

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn command_build_upload_help() {
    TestManager::new().register_trycmd_test("build/build-upload-help-macos.trycmd");
}

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
#[test]
fn command_build_upload_help() {
    TestManager::new().register_trycmd_test("build/build-upload-help-not-macos.trycmd");
}

#[test]
fn command_build_upload_no_token() {
    TestManager::new().register_trycmd_test("build/build-upload-apk-no-token.trycmd");
}

#[test]
fn command_build_upload_no_path() {
    TestManager::new().register_trycmd_test("build/build-upload-no-path.trycmd");
}

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
#[test]
fn command_build_upload_ipa_not_arm64() {
    TestManager::new()
        .register_trycmd_test("build/build-upload-ipa-not-arm64.trycmd")
        .with_default_token();
}

#[test]
fn command_build_upload_invalid_aab() {
    TestManager::new()
        .assert_cmd(vec![
            "build",
            "upload",
            "tests/integration/_fixtures/build/invalid_aab.aab",
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Failure);
}

#[test]
fn command_build_upload_invalid_apk() {
    TestManager::new()
        .assert_cmd(vec![
            "build",
            "upload",
            "tests/integration/_fixtures/build/invalid_apk.apk",
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Failure);
}

#[test]
fn command_build_upload_invalid_xcarchive() {
    TestManager::new()
        .assert_cmd(vec![
            "build",
            "upload",
            "tests/integration/_fixtures/build/invalid_xcarchive",
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Failure);
}

#[test]
fn command_build_upload_invalid_ipa() {
    TestManager::new()
        .assert_cmd(vec![
            "build",
            "upload",
            "tests/integration/_fixtures/build/invalid.ipa",
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Failure);
}

#[test]
fn command_build_upload_apk_all_uploaded() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/")
                .with_response_file("build/get-chunk-upload.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "POST",
                "/api/0/projects/wat-org/wat-project/files/preprodartifacts/assemble/",
            )
            .with_response_body(r#"{"state":"ok","missingChunks":[],"artifactUrl":"https://sentry.io/wat-org/preprod/wat-project/42"}"#),
        )
        .register_trycmd_test("build/build-upload-apk-all-uploaded.trycmd")
        .with_default_token();
}

#[test]
fn command_build_upload_apk_invlid_sha() {
    TestManager::new().register_trycmd_test("build/build-invalid-*-sha.trycmd");
}

#[test]
/// This test simulates a full chunk upload (with only one chunk).
/// It verifies that the Sentry CLI makes the expected API calls to the chunk upload endpoint
/// and that the data sent to the chunk upload endpoint is exactly as expected.
/// It also verifies that the correct calls are made to the assemble endpoint.
fn command_build_upload_apk_chunked() {
    let is_first_assemble_call = AtomicBool::new(true);

    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/")
                .with_response_file("build/get-chunk-upload.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new("POST", "/api/0/organizations/wat-org/chunk-upload/")
                .with_response_fn(move |request| {
                    let boundary = chunk_upload::boundary_from_request(request)
                        .expect("content-type header should be a valid multipart/form-data header");

                    let body = request.body().expect("body should be readable");

                    let decompressed = chunk_upload::decompress_chunks(body, boundary)
                        .expect("chunks should be valid gzip data");

                    assert_eq!(decompressed.len(), 1, "expected exactly one chunk");

                    // The CLI wraps the APK in a zip bundle with metadata.
                    // Verify the bundle is a valid zip containing the APK.
                    let chunk = decompressed.first().unwrap();
                    let cursor = std::io::Cursor::new(chunk);
                    let mut archive =
                        zip::ZipArchive::new(cursor).expect("chunk should be a valid zip");
                    let apk_entry = archive
                        .by_name("apk.apk")
                        .expect("bundle should contain the APK file");
                    let expected_size =
                        std::fs::metadata("tests/integration/_fixtures/build/apk.apk")
                            .expect("fixture file should exist")
                            .len();
                    assert_eq!(
                        apk_entry.size(),
                        expected_size,
                        "APK size in bundle should match the fixture"
                    );

                    vec![] // Client does not expect a response body
                }),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "POST",
                "/api/0/projects/wat-org/wat-project/files/preprodartifacts/assemble/",
            )
            .with_header_matcher("content-type", "application/json")
            .with_response_fn(move |_| {
                if is_first_assemble_call.swap(false, Ordering::Relaxed) {
                    r#"{
                        "state": "created",
                        "missingChunks": ["7138c09b474a5c84ac60e1b145855bf6dcc88913"]
                    }"#
                } else {
                    r#"{
                        "state": "ok",
                        "missingChunks": [],
                        "artifactUrl": "http://sentry.io/wat-org/preprod/wat-project/42"
                    }"#
                }
                .into()
            })
            .expect(2),
        )
        .register_trycmd_test("build/build-upload-apk.trycmd")
        // We override the version in the metadata field to ensure a consistent checksum
        // for the uploaded files.
        .env("SENTRY_CLI_INTEGRATION_TEST_VERSION_OVERRIDE", "0.0.0-test")
        .with_default_token();
}

#[test]
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
/// This test simulates a full chunk upload for an IPA file (with only one chunk).
/// It verifies that the Sentry CLI makes the expected API calls to the chunk upload endpoint
/// and that the data sent to the chunk upload endpoint is exactly as expected.
/// It also verifies that the correct calls are made to the assemble endpoint.
fn command_build_upload_ipa_chunked() {
    let is_first_assemble_call = AtomicBool::new(true);

    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/")
                .with_response_file("build/get-chunk-upload.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new("POST", "/api/0/organizations/wat-org/chunk-upload/")
                .with_response_fn(move |request| {
                    let content_type_headers = request.header("content-type");
                    assert_eq!(
                        content_type_headers.len(),
                        1,
                        "content-type header should be present exactly once, found {} times",
                        content_type_headers.len()
                    );
                    vec![] // Client does not expect a response body
                }),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "POST",
                "/api/0/projects/wat-org/wat-project/files/preprodartifacts/assemble/",
            )
            .with_header_matcher("content-type", "application/json")
            .with_response_fn(move |_| {
                if is_first_assemble_call.swap(false, Ordering::Relaxed) {
                    r#"{
                        "state": "created",
                        "missingChunks": ["ecf0e7cb306f29b21189f49d0879bd85aa4be146"]
                    }"#
                } else {
                    r#"{
                        "state": "ok",
                        "missingChunks": [],
                        "artifactUrl": "http://sentry.io/wat-org/preprod/wat-project/some-text-id"
                    }"#
                }
                .into()
            })
            .expect(2),
        )
        .register_trycmd_test("build/build-upload-ipa.trycmd")
        // We override the version in the metadata field to ensure a consistent checksum
        // for the uploaded files.
        .env("SENTRY_CLI_INTEGRATION_TEST_VERSION_OVERRIDE", "0.0.0-test")
        .with_default_token();
}

#[test]
fn command_build_upload_empty_shas() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/")
                .with_response_file("build/get-chunk-upload.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "POST",
                "/api/0/projects/wat-org/wat-project/files/preprodartifacts/assemble/",
            )
            .with_response_fn(move |req| {
                let body = req.body().expect("body should be readable");
                let json: serde_json::Value =
                    serde_json::from_slice(body).expect("body should be valid JSON");
                assert!(
                    json.get("head_sha").is_none(),
                    "head_sha should not be present"
                );
                assert!(
                    json.get("base_sha").is_none(),
                    "base_sha should not be present"
                );

                serde_json::json!({
                    "state": "created",
                    "missingChunks": [],
                    "artifactUrl": "http://sentry.io/wat-org/preprod/wat-project/42"
                })
                .to_string()
                .into()
            }),
        )
        .register_trycmd_test("build/build-upload-empty-shas.trycmd")
        .with_default_token();
}

/// Verify that all string-based arguments to `build upload` can be set to an empty string,
/// and that setting to empty string results in the corresponding fields being omitted from
/// the request body.
#[test]
fn command_build_upload_empty_refs() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/")
                .with_response_file("build/get-chunk-upload.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "POST",
                "/api/0/projects/wat-org/wat-project/files/preprodartifacts/assemble/",
            )
            .with_response_fn(move |req| {
                let body = req.body().expect("body should be readable");
                let json: serde_json::Value =
                    serde_json::from_slice(body).expect("body should be valid JSON");

                assert!(json.get("provider").is_none());
                assert!(json.get("head_repo_name").is_none());
                assert!(json.get("base_repo_name").is_none());
                assert!(json.get("head_ref").is_none());
                assert!(json.get("base_ref").is_none());

                serde_json::json!({
                    "state": "created",
                    "missingChunks": [],
                    "artifactUrl": "http://sentry.io/wat-org/preprod/wat-project/42"
                })
                .to_string()
                .into()
            }),
        )
        .assert_cmd([
            "build",
            "upload",
            "tests/integration/_fixtures/build/apk.apk",
            "--vcs-provider",
            "",
            "--head-repo-name",
            "",
            "--base-repo-name",
            "",
            "--head-ref",
            "",
            "--base-ref",
            "",
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Success);
}
