use std::fs;
use std::sync::atomic::{AtomicU8, Ordering};

use mockito::Matcher;
use serde_json::json;

use crate::integration::test_utils::{chunk_upload, AssertCommand};
use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_proguard() {
    TestManager::new().register_trycmd_test("proguard/*.trycmd");
}

#[test]
fn command_proguard_upload_no_upload_no_auth_token() {
    TestManager::new().register_trycmd_test("proguard/proguard-upload-no-upload.trycmd");
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
            .with_matcher(Matcher::Json(json!({
                "297ecd9143fc2882e4b6758c1ccd13ea82930eeb": {
                    "name": "/proguard/c038584d-c366-570c-ad1e-034fa0d194d7.txt",
                    "chunks": ["297ecd9143fc2882e4b6758c1ccd13ea82930eeb"]
                }
            })))
            .with_response_body(
                json!({
                    "297ecd9143fc2882e4b6758c1ccd13ea82930eeb": {
                        "state": "ok",
                        "detail": null,
                        "missingChunks": [],
                        "dif": {
                            "id": "12",
                            "uuid": "c038584d-c366-570c-ad1e-034fa0d194d7",
                            "debugId": "c038584d-c366-570c-ad1e-034fa0d194d7",
                            "codeId": null,
                            "cpuName": "any",
                            "objectName": "proguard-mapping",
                            "symbolType": "proguard",
                            "headers": {"Content-Type": "text/x-proguard+plain"},
                            "size": 155,
                            "sha1": "297ecd9143fc2882e4b6758c1ccd13ea82930eeb",
                            "dateCreated": "1776-07-04T12:00:00.000Z",
                            "data": {"features": ["mapping"]}
                        }
                    }
                })
                .to_string(),
            ),
        )
        .assert_cmd([
            "proguard",
            "upload",
            "tests/integration/_fixtures/proguard/upload/mapping.txt",
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Success)
}

#[test]
fn chunk_upload_needs_upload() {
    const EXPECTED_CHUNKS_BOUNDARY: &str = "------------------------w2uOUUnuLEYTmQorc0ix48";

    let call_count = AtomicU8::new(0);
    let expected_chunk_body = fs::read(
        "tests/integration/_expected_requests/proguard/upload/chunk_upload_needs_upload.bin",
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
            .with_matcher(Matcher::Json(json!({
                "297ecd9143fc2882e4b6758c1ccd13ea82930eeb": {
                    "name": "/proguard/c038584d-c366-570c-ad1e-034fa0d194d7.txt",
                    "chunks": ["297ecd9143fc2882e4b6758c1ccd13ea82930eeb"]
                }
            })))
            .with_response_fn(move |_| {
                match call_count.fetch_add(1, Ordering::Relaxed) {
                    0 => {
                        // First call: The file is not found since it still needs to be uploaded.
                        json!({
                            "297ecd9143fc2882e4b6758c1ccd13ea82930eeb": {
                                "state": "not_found",
                                "missingChunks": ["297ecd9143fc2882e4b6758c1ccd13ea82930eeb"]
                            }
                        })
                        .to_string()
                        .into_bytes()
                    }
                    1 => {
                        // Second call: The file has been uploaded, assemble job created.
                        json!({
                            "297ecd9143fc2882e4b6758c1ccd13ea82930eeb": {
                                "state": "created",
                                "missingChunks": []
                            }
                        })
                        .to_string()
                        .into_bytes()
                    }
                    n => panic!(
                        "Only 2 calls to the assemble endpoint expected, but there were {}.",
                        n + 1
                    ),
                }
            })
            .expect(2),
        )
        .assert_cmd([
            "proguard",
            "upload",
            "tests/integration/_fixtures/proguard/upload/mapping.txt",
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Success)
}

#[test]
fn chunk_upload_two_files() {
    const EXPECTED_CHUNKS_BOUNDARY: &str = "------------------------HNdDRjCgjkRtu3COUTCcJV";

    let call_count = AtomicU8::new(0);
    let expected_chunk_body =
        fs::read("tests/integration/_expected_requests/proguard/upload/chunk_upload_two_files.bin")
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
                [
                    Matcher::Json(json!({
                        "297ecd9143fc2882e4b6758c1ccd13ea82930eeb": {
                            "name": "/proguard/c038584d-c366-570c-ad1e-034fa0d194d7.txt",
                            "chunks": ["297ecd9143fc2882e4b6758c1ccd13ea82930eeb"]
                        },
                        "e5329624a8d06e084941f133c75b5874f793ee7c": {
                            "name": "/proguard/747e1d76-509b-5225-8a5b-db7b7d4067d4.txt",
                            "chunks": ["e5329624a8d06e084941f133c75b5874f793ee7c"]
                        }
                    })),
                    Matcher::Json(json!({
                        "e5329624a8d06e084941f133c75b5874f793ee7c": {
                            "name": "/proguard/747e1d76-509b-5225-8a5b-db7b7d4067d4.txt",
                            "chunks": ["e5329624a8d06e084941f133c75b5874f793ee7c"]
                        },
                        "297ecd9143fc2882e4b6758c1ccd13ea82930eeb": {
                            "name": "/proguard/c038584d-c366-570c-ad1e-034fa0d194d7.txt",
                            "chunks": ["297ecd9143fc2882e4b6758c1ccd13ea82930eeb"]
                        }
                    })),
                ]
                .into(),
            ))
            .with_response_fn(move |_| {
                match call_count.fetch_add(1, Ordering::Relaxed) {
                    0 => {
                        // First call: The file is not found since it still needs to be uploaded.
                        json!({
                            "297ecd9143fc2882e4b6758c1ccd13ea82930eeb": {
                                "state": "not_found",
                                "missingChunks": ["297ecd9143fc2882e4b6758c1ccd13ea82930eeb"]
                            },
                            "e5329624a8d06e084941f133c75b5874f793ee7c": {
                                "state": "not_found",
                                "missingChunks": ["e5329624a8d06e084941f133c75b5874f793ee7c"]
                            }
                        })
                        .to_string()
                        .into_bytes()
                    }
                    1 => {
                        // Second call: The file has been uploaded, assemble job created.
                        json!({
                            "297ecd9143fc2882e4b6758c1ccd13ea82930eeb": {
                                "state": "created",
                                "missingChunks": []
                            },
                            "e5329624a8d06e084941f133c75b5874f793ee7c": {
                                "state": "created",
                                "missingChunks": []
                            }
                        })
                        .to_string()
                        .into_bytes()
                    }
                    n => panic!(
                        "Only 2 calls to the assemble endpoint expected, but there were {}.",
                        n + 1
                    ),
                }
            })
            .expect(2),
        )
        .assert_cmd([
            "proguard",
            "upload",
            "tests/integration/_fixtures/proguard/upload/mapping.txt",
            "tests/integration/_fixtures/proguard/upload/mapping-2.txt",
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Success)
}
