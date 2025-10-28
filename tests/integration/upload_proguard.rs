use std::sync::atomic::{AtomicU8, Ordering};
use std::{fs, str};

use mockito::Matcher;

use crate::integration::test_utils::{chunk_upload, AssertCommand};
use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_upload_proguard() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("POST", "/api/0/projects/wat-org/wat-project/files/dsyms/")
                .with_response_body("[]"),
        )
        .register_trycmd_test("upload_proguard/*.trycmd")
        .with_default_token();
}

#[test]
fn command_upload_proguard_no_upload_no_auth_token() {
    TestManager::new().register_trycmd_test("upload_proguard/upload_proguard-no-upload.trycmd");
}

#[test]
fn chunk_upload_already_there() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/")
                .with_response_file("debug_files/get-chunk-upload.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "POST",
                "/api/0/projects/wat-org/wat-project/files/difs/assemble/",
            )
            .with_header_matcher("content-type", "application/json")
            .with_matcher(
                "{\
                    \"297ecd9143fc2882e4b6758c1ccd13ea82930eeb\":{\
                        \"name\":\"/proguard/c038584d-c366-570c-ad1e-034fa0d194d7.txt\",\
                        \"chunks\":[\"297ecd9143fc2882e4b6758c1ccd13ea82930eeb\"]\
                    }\
                }",
            )
            .with_response_body(
                "{\
                    \"297ecd9143fc2882e4b6758c1ccd13ea82930eeb\":{\
                        \"state\":\"ok\",\
                        \"detail\":null,\
                        \"missingChunks\":[],\
                        \"dif\":{\
                            \"id\":\"12\",\
                            \"uuid\":\"c038584d-c366-570c-ad1e-034fa0d194d7\",\
                            \"debugId\":\"c038584d-c366-570c-ad1e-034fa0d194d7\",\
                            \"codeId\":null,\
                            \"cpuName\":\"any\",\
                            \"objectName\":\"proguard-mapping\",\
                            \"symbolType\":\"proguard\",\
                            \"headers\":{\"Content-Type\":\"text/x-proguard+plain\"},\
                            \"size\":155,\
                            \"sha1\":\"297ecd9143fc2882e4b6758c1ccd13ea82930eeb\",\
                            \"dateCreated\":\"1776-07-04T12:00:00.000Z\",\
                            \"data\":{\"features\":[\"mapping\"]}\
                        }\
                    }\
                }",
            ),
        )
        .assert_cmd([
            "upload-proguard",
            "tests/integration/_fixtures/upload_proguard/mapping.txt",
        ])
        .with_default_token()
        .env("SENTRY_EXPERIMENTAL_PROGUARD_CHUNK_UPLOAD", "1")
        .run_and_assert(AssertCommand::Success)
}

#[test]
fn chunk_upload_needs_upload() {
    const EXPECTED_CHUNKS_BOUNDARY: &str = "------------------------w2uOUUnuLEYTmQorc0ix48";

    let call_count = AtomicU8::new(0);
    let expected_chunk_body = fs::read(
        "tests/integration/_expected_requests/upload_proguard/chunk_upload_needs_upload.bin",
    )
    .expect("expected chunk upload request file should be readable");

    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/")
                .with_response_file("debug_files/get-chunk-upload.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new("POST", "/api/0/organizations/wat-org/chunk-upload/")
                .with_response_fn(move |request| {
                    let boundary = chunk_upload::boundary_from_request(request)
                        .expect("content-type header should be a valid multipart/form-data header");

                    let body = request.body().expect("body should be readable");

                    let chunks = chunk_upload::split_chunk_body(body, boundary)
                        .expect("body should be a valid multipart/form-data body");

                    let expected_chunks = chunk_upload::split_chunk_body(
                        &expected_chunk_body,
                        EXPECTED_CHUNKS_BOUNDARY,
                    )
                    .expect("expected body is valid multipart form data");

                    // Using assert! because in case of failure, the output with assert_eq!
                    // is too long to be useful.
                    assert_eq!(
                        chunks, expected_chunks,
                        "Uploaded chunks differ from the expected chunks"
                    );

                    vec![]
                }),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "POST",
                "/api/0/projects/wat-org/wat-project/files/difs/assemble/",
            )
            .with_header_matcher("content-type", "application/json")
            .with_matcher(
                "{\
                    \"297ecd9143fc2882e4b6758c1ccd13ea82930eeb\":{\
                        \"name\":\"/proguard/c038584d-c366-570c-ad1e-034fa0d194d7.txt\",\
                        \"chunks\":[\"297ecd9143fc2882e4b6758c1ccd13ea82930eeb\"]\
                    }\
                }",
            )
            .with_response_fn(move |_| {
                match call_count.fetch_add(1, Ordering::Relaxed) {
                    0 => {
                        // First call: The file is not found since it still needs to be uploaded.
                        "{\
                            \"297ecd9143fc2882e4b6758c1ccd13ea82930eeb\":{\
                                \"state\":\"not_found\",\
                                \"missingChunks\":[\"297ecd9143fc2882e4b6758c1ccd13ea82930eeb\"]\
                            }\
                        }"
                    }
                    1 => {
                        // Second call: The file has been uploaded, assemble job created.
                        "{\
                            \"297ecd9143fc2882e4b6758c1ccd13ea82930eeb\":{\
                                \"state\":\"created\",\
                                \"missingChunks\":[]\
                            }\
                        }"
                    }
                    n => panic!(
                        "Only 2 calls to the assemble endpoint expected, but there were {}.",
                        n + 1
                    ),
                }
                .into()
            })
            .expect(2),
        )
        .assert_cmd([
            "upload-proguard",
            "tests/integration/_fixtures/upload_proguard/mapping.txt",
        ])
        .with_default_token()
        .env("SENTRY_EXPERIMENTAL_PROGUARD_CHUNK_UPLOAD", "1")
        .run_and_assert(AssertCommand::Success)
}

#[test]
fn chunk_upload_two_files() {
    const EXPECTED_CHUNKS_BOUNDARY: &str = "------------------------HNdDRjCgjkRtu3COUTCcJV";

    let call_count = AtomicU8::new(0);
    let expected_chunk_body =
        fs::read("tests/integration/_expected_requests/upload_proguard/chunk_upload_two_files.bin")
            .expect("expected chunk upload request file should be readable");

    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/")
                .with_response_file("debug_files/get-chunk-upload.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new("POST", "/api/0/organizations/wat-org/chunk-upload/")
                .with_response_fn(move |request| {
                    let boundary = chunk_upload::boundary_from_request(request)
                        .expect("content-type header should be a valid multipart/form-data header");

                    let body = request.body().expect("body should be readable");

                    let chunks = chunk_upload::split_chunk_body(body, boundary)
                        .expect("body should be a valid multipart/form-data body");

                    let expected_chunks = chunk_upload::split_chunk_body(
                        &expected_chunk_body,
                        EXPECTED_CHUNKS_BOUNDARY,
                    )
                    .expect("expected body is valid multipart form data");

                    // Using assert! because in case of failure, the output with assert_eq!
                    // is too long to be useful.
                    assert_eq!(
                        chunks, expected_chunks,
                        "Uploaded chunks differ from the expected chunks"
                    );

                    vec![]
                }),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "POST",
                "/api/0/projects/wat-org/wat-project/files/difs/assemble/",
            )
            .with_header_matcher("content-type", "application/json")
            .with_matcher(Matcher::AnyOf(
                // The ordering of the two files in the assemble request changes.
                // Here, we match on either ordering.
                [
                    "{\
                        \"297ecd9143fc2882e4b6758c1ccd13ea82930eeb\":{\
                            \"name\":\"/proguard/c038584d-c366-570c-ad1e-034fa0d194d7.txt\",\
                            \"chunks\":[\"297ecd9143fc2882e4b6758c1ccd13ea82930eeb\"]\
                        },\
                        \"e5329624a8d06e084941f133c75b5874f793ee7c\":{\
                            \"name\":\"/proguard/747e1d76-509b-5225-8a5b-db7b7d4067d4.txt\",\
                            \"chunks\":[\"e5329624a8d06e084941f133c75b5874f793ee7c\"]\
                        }\
                    }"
                    .into(),
                    "{\
                        \"e5329624a8d06e084941f133c75b5874f793ee7c\":{\
                            \"name\":\"/proguard/747e1d76-509b-5225-8a5b-db7b7d4067d4.txt\",\
                            \"chunks\":[\"e5329624a8d06e084941f133c75b5874f793ee7c\"]\
                        },\
                        \"297ecd9143fc2882e4b6758c1ccd13ea82930eeb\":{\
                            \"name\":\"/proguard/c038584d-c366-570c-ad1e-034fa0d194d7.txt\",\
                            \"chunks\":[\"297ecd9143fc2882e4b6758c1ccd13ea82930eeb\"]\
                        }\
                    }"
                    .into(),
                ]
                .into(),
            ))
            .with_response_fn(move |_| {
                match call_count.fetch_add(1, Ordering::Relaxed) {
                    0 => {
                        // First call: The file is not found since it still needs to be uploaded.
                        "{\
                            \"297ecd9143fc2882e4b6758c1ccd13ea82930eeb\":{\
                                \"state\":\"not_found\",\
                                \"missingChunks\":[\"297ecd9143fc2882e4b6758c1ccd13ea82930eeb\"]\
                            },\
                            \"e5329624a8d06e084941f133c75b5874f793ee7c\":{\
                                \"state\":\"not_found\",\
                                \"missingChunks\":[\"e5329624a8d06e084941f133c75b5874f793ee7c\"]\
                            }\
                        }"
                    }
                    1 => {
                        // Second call: The file has been uploaded, assemble job created.
                        "{\
                            \"297ecd9143fc2882e4b6758c1ccd13ea82930eeb\":{\
                                \"state\":\"created\",\
                                \"missingChunks\":[]\
                            },\
                            \"e5329624a8d06e084941f133c75b5874f793ee7c\":{\
                                \"state\":\"created\",\
                                \"missingChunks\":[]\
                            }\
                        }"
                    }
                    n => panic!(
                        "Only 2 calls to the assemble endpoint expected, but there were {}.",
                        n + 1
                    ),
                }
                .into()
            })
            .expect(2),
        )
        .assert_cmd([
            "upload-proguard",
            "tests/integration/_fixtures/upload_proguard/mapping.txt",
            "tests/integration/_fixtures/upload_proguard/mapping-2.txt",
        ])
        .with_default_token()
        .env("SENTRY_EXPERIMENTAL_PROGUARD_CHUNK_UPLOAD", "1")
        .run_and_assert(AssertCommand::Success)
}
