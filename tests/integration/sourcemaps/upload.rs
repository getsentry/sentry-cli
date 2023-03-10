use crate::integration::{mock_endpoint, register_test, EndpointOptions};
use mockito::{server_url, Mock};

enum ServerBehavior {
    Legacy,
    Modern,
}

// Endpoints need to be bound, as they need to live long enough for test to finish
fn mock_common_upload_endpoints(behavior: ServerBehavior) -> Vec<Mock> {
    let (accept, release_request_count, assemble_endpoint) = match behavior {
        ServerBehavior::Legacy => (
            "\"release_files\"",
            2,
            "/api/0/organizations/wat-org/releases/wat-release/assemble/",
        ),
        ServerBehavior::Modern => (
            "\"release_files\", \"artifact_bundles\"",
            0,
            "/api/0/organizations/wat-org/artifactbundle/assemble/",
        ),
    };
    let chunk_upload_response = format!(
        "{{
            \"url\": \"{}/api/0/organizations/wat-org/chunk-upload/\",
            \"chunkSize\": 8388608,
            \"chunksPerRequest\": 64,
            \"maxRequestSize\": 33554432,
            \"concurrency\": 8,
            \"hashAlgorithm\": \"sha1\",
            \"accept\": [{}]
          }}",
        server_url(),
        accept,
    );

    vec![
        // bad endpoint, bad endpoint
        mock_endpoint(
            EndpointOptions::new("POST", "/api/0/projects/wat-org/wat-project/releases/", 208)
                .with_response_file("releases/get-release.json"),
        )
        .expect_at_least(release_request_count)
        .expect_at_most(release_request_count),
        mock_endpoint(
            EndpointOptions::new("GET", "/api/0/organizations/wat-org/chunk-upload/", 200)
                .with_response_body(chunk_upload_response),
        ),
        mock_endpoint(
            EndpointOptions::new("POST", "/api/0/organizations/wat-org/chunk-upload/", 200)
                .with_response_body("[]"),
        ),
        mock_endpoint(
            EndpointOptions::new("POST", assemble_endpoint, 200)
                .with_response_body(r#"{"state":"created","missingChunks":[]}"#),
        ),
    ]
}

fn assert_endpoints(mocks: &[Mock]) {
    mocks[0].assert();
}

#[test]
fn command_sourcemaps_upload_help() {
    register_test("sourcemaps/sourcemaps-upload-help.trycmd");
}

#[test]
fn command_sourcemaps_upload() {
    register_test("sourcemaps/sourcemaps-upload.trycmd");
}

#[test]
fn command_sourcemaps_upload_successfully_upload_file() {
    let upload_endpoints = mock_common_upload_endpoints(ServerBehavior::Legacy);
    let _files = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/wat-release/files/?cursor=",
            200,
        )
        .with_response_body("[]"),
    );

    register_test("sourcemaps/sourcemaps-upload-successfully-upload-file.trycmd");
    assert_endpoints(&upload_endpoints);
}

#[test]
fn command_sourcemaps_upload_skip_already_uploaded() {
    let upload_endpoints = mock_common_upload_endpoints(ServerBehavior::Legacy);
    let _files = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/wat-release/files/?cursor=&checksum=38ed853073df85147960ea3a5bced6170ec389b0&checksum=f3673e2cea68bcb86bb74254a9efaa381d74929f",
            200,
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
    );

    register_test("sourcemaps/sourcemaps-upload-skip-already-uploaded.trycmd");
    assert_endpoints(&upload_endpoints);
}

#[test]
fn command_sourcemaps_upload_no_dedupe() {
    let upload_endpoints = mock_common_upload_endpoints(ServerBehavior::Legacy);
    let _files = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/wat-release/files/?cursor=",
            200,
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
    );

    register_test("sourcemaps/sourcemaps-upload-no-dedupe.trycmd");
    assert_endpoints(&upload_endpoints);
}

#[test]
fn command_sourcemaps_upload_modern() {
    let upload_endpoints = mock_common_upload_endpoints(ServerBehavior::Modern);
    register_test("sourcemaps/sourcemaps-upload-modern.trycmd");
    assert_endpoints(&upload_endpoints);
}

#[test]
fn command_releases_files_upload_sourcemap() {
    let upload_endpoints = mock_common_upload_endpoints(ServerBehavior::Legacy);
    let _files = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/wat-release/files/?cursor=",
            200,
        )
        .with_response_body("[]"),
    );
    register_test("releases/releases-files-upload-sourcemaps.trycmd");
    assert_endpoints(&upload_endpoints);
}
