use crate::integration::{mock_endpoint, register_test, EndpointOptions};
use mockito::{server_url, Mock};

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
    let _upload_endpoints = mock_common_upload_endpoints();
    let _files = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/wat-release/files/?cursor=",
            200,
        )
        .with_response_body("[]"),
    );

    register_test("sourcemaps/sourcemaps-upload-successfully-upload-file.trycmd");
}

#[test]
fn command_sourcemaps_upload_skip_already_uploaded() {
    let _upload_endpoints = mock_common_upload_endpoints();
    let _files = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/wat-release/files/?cursor=",
            200,
        )
        .with_response_body(
            r#"[{
                "id": "1337",
                "name": "~/bundle.min.js.map",
                "headers": {},
                "size": 1522,
                "sha1": "38ed853073df85147960ea3a5bced6170ec389b0",
                "dateCreated": "2022-05-12T11:08:01.496220Z"
            }]"#,
        ),
    );

    register_test("sourcemaps/sourcemaps-upload-skip-already-uploaded.trycmd");
}

// Endpoints need to be bound, as they need to live long enough for test to finish
fn mock_common_upload_endpoints() -> Vec<Mock> {
    let chunk_upload_response = format!(
        "{{
            \"url\": \"{}/api/0/organizations/wat-org/chunk-upload/\",
            \"chunkSize\": 8388608,
            \"chunksPerRequest\": 64,
            \"maxRequestSize\": 33554432,
            \"concurrency\": 8,
            \"hashAlgorithm\": \"sha1\",
            \"accept\": [
              \"release_files\"
            ]
          }}",
        server_url()
    );

    vec![
        mock_endpoint(
            EndpointOptions::new("POST", "/api/0/projects/wat-org/wat-project/releases/", 208)
                .with_response_file("releases/get-release.json"),
        ),
        mock_endpoint(
            EndpointOptions::new("GET", "/api/0/organizations/wat-org/chunk-upload/", 200)
                .with_response_body(chunk_upload_response),
        ),
        mock_endpoint(
            EndpointOptions::new("POST", "/api/0/organizations/wat-org/chunk-upload/", 200)
                .with_response_body("[]"),
        ),
        mock_endpoint(
            EndpointOptions::new(
                "POST",
                "/api/0/organizations/wat-org/releases/wat-release/assemble/",
                200,
            )
            .with_response_body(r#"{"state":"created","missingChunks":[]}"#),
        ),
    ]
}
