use crate::integration::{AssertCommand, MockEndpointBuilder, TestManager};
use regex::bytes::Regex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use std::{fs, str};

#[test]
fn command_mobile_app_upload_help() {
    TestManager::new().register_trycmd_test("mobile_app/mobile_app-upload-help.trycmd");
}

#[test]
fn command_mobile_app_upload_no_token() {
    TestManager::new().register_trycmd_test("mobile_app/mobile_app-upload-apk-no-token.trycmd");
}

#[test]
fn command_mobile_app_upload_invalid_aab() {
    TestManager::new()
        .assert_cmd(vec![
            "mobile-app",
            "upload",
            "tests/integration/_fixtures/mobile_app/invalid_aab.aab",
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Failure);
}

#[test]
fn command_mobile_app_upload_invalid_apk() {
    TestManager::new()
        .assert_cmd(vec![
            "mobile-app",
            "upload",
            "tests/integration/_fixtures/mobile_app/invalid_apk.apk",
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Failure);
}

#[test]
fn command_mobile_app_upload_invalid_xcarchive() {
    TestManager::new()
        .assert_cmd(vec![
            "mobile-app",
            "upload",
            "tests/integration/_fixtures/mobile_app/invalid_xcarchive",
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Failure);
}

#[test]
fn command_mobile_app_upload_apk_all_uploaded() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/")
                .with_response_file("mobile_app/get-chunk-upload.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "POST",
                "/api/0/projects/wat-org/wat-project/files/preprodartifacts/assemble/",
            )
            .with_response_body(r#"{"state":"ok","missingChunks":[]}"#),
        )
        .register_trycmd_test("mobile_app/mobile_app-upload-apk-all-uploaded.trycmd")
        .with_default_token();
}

/// This regex is used to extract the boundary from the content-type header.
/// We need to match the boundary, since it changes with each request.
/// The regex matches the format as specified in
/// https://www.w3.org/Protocols/rfc1341/7_2_Multipart.html.
static CONTENT_TYPE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^multipart\/form-data; boundary=(?<boundary>[\w'\(\)+,\-\.\/:=? ]{0,69}[\w'\(\)+,\-\.\/:=?])$"#
    )
    .expect("Regex is valid")
});

#[test]
/// This test simulates a full chunk upload (with only one chunk).
/// It verifies that the Sentry CLI makes the expected API calls to the chunk upload endpoint
/// and that the data sent to the chunk upload endpoint is exactly as expected.
/// It also verifies that the correct calls are made to the assemble endpoint.
fn command_mobile_app_upload_apk_chunked() {
    let is_first_assemble_call = AtomicBool::new(true);
    let expected_chunk_body =
        fs::read("tests/integration/_expected_requests/mobile_app/apk_chunk.bin")
            .expect("expected chunk body file should be present");

    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/")
                .with_response_file("mobile_app/get-chunk-upload.json"),
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
                    let content_type = content_type_headers[0].as_bytes();
                    let boundary = CONTENT_TYPE_REGEX
                        .captures(content_type)
                        .expect("content-type should match regex")
                        .name("boundary")
                        .expect("boundary should be present")
                        .as_bytes();
                    let boundary_str = str::from_utf8(boundary).expect("boundary should be valid utf-8");
                    let boundary_escaped = regex::escape(boundary_str);
                    let body_regex = Regex::new(&format!(
                        r#"^--{boundary_escaped}(?<chunk_body>(?s-u:.)*?)--{boundary_escaped}--\s*$"#
                    ))
                    .expect("regex should be valid");
                    let body = request.body().expect("body should be readable");
                    let chunk_body = body_regex
                        .captures(body)
                        .expect("body should match regex")
                        .name("chunk_body")
                        .expect("chunk_body section should be present")
                        .as_bytes();
                    assert_eq!(chunk_body, expected_chunk_body);
                    vec![] // Client does not expect a response body
                }),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "POST",
                "/api/0/projects/wat-org/wat-project/files/preprodartifacts/assemble/",
            )
            .with_header_matcher("content-type", "application/json")
            .with_matcher(r#"{"checksum":"18e40e6e932d0b622d631e887be454cc2003dbb5","chunks":["18e40e6e932d0b622d631e887be454cc2003dbb5"],"git_sha":"test_sha"}"#)
            .with_response_fn(move |_| {
                if is_first_assemble_call.swap(false, Ordering::Relaxed) {
                    r#"{
                        "state": "created",
                        "missingChunks": ["18e40e6e932d0b622d631e887be454cc2003dbb5"]
                    }"#
                } else {
                    r#"{
                        "state": "ok",
                        "missingChunks": []
                    }"#
                }
                .into()
            })
            .expect(2),
        )
        .register_trycmd_test("mobile_app/mobile_app-upload-apk.trycmd")
        .with_default_token();
}
