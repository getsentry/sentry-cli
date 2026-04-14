use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Deserialize;
use sha1_smol::Sha1;

use crate::integration::{chunk_upload, AssertCommand, MockEndpointBuilder, TestManager};

#[test]
fn command_debug_files_upload() {
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
            .with_response_file("debug_files/post-difs-assemble.json"),
        )
        .register_trycmd_test("debug_files/upload/debug_files-upload.trycmd")
        .with_default_token();
}

#[test]
fn command_debug_files_upload_pdb() {
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
            .with_response_body(
                r#"{
                "5f81d6becc51980870acc9f6636ab53d26160763": {
                    "state": "ok",
                    "missingChunks": []
                }
            }"#,
            ),
        )
        .register_trycmd_test("debug_files/upload/debug_files-upload-pdb.trycmd")
        .register_trycmd_test("debug_files/upload/debug_files-upload-pdb-include-sources.trycmd")
        .with_default_token();
}

#[test]
fn command_debug_files_upload_pdb_embedded_sources() {
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
            .with_response_body(
                r#"{
                "50dd9456dc89cdbc767337da512bdb36b15db6b2": {
                    "state": "ok",
                    "missingChunks": []
                }
            }"#,
            ),
        )
        .register_trycmd_test("debug_files/upload/debug_files-upload-pdb-embedded-sources.trycmd")
        .with_default_token();
}

#[test]
fn command_debug_files_upload_dll_embedded_ppdb_with_sources() {
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
            .with_response_body(
                r#"{
                "fc1c9e58a65bd4eaf973bbb7e7a7cc01bfdaf15e": {
                    "state": "ok",
                    "missingChunks": []
                }
            }"#,
            ),
        )
        .register_trycmd_test(
            "debug_files/upload/debug_files-upload-dll-embedded-ppdb-with-sources.trycmd",
        )
        .with_default_token();
}

#[test]
fn command_debug_files_upload_mixed_embedded_sources() {
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
            .with_response_body(
                r#"{
                    "21b76b717dbbd8c89e42d92b29667ac87aa3c124": {
                        "state": "ok",
                        "missingChunks": []
                    }
                }"#,
            ),
        )
        .register_trycmd_test("debug_files/upload/debug_files-upload-mixed-embedded-sources.trycmd")
        .with_default_token();
}

#[test]
fn command_debug_files_upload_no_upload() {
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
            .with_response_file("debug_files/post-difs-assemble.json"),
        )
        .register_trycmd_test("debug_files/upload/debug_files-upload-no-upload.trycmd")
        .with_default_token();
}

#[test]
/// This test ensures that the correct initial call to the debug files assemble endpoint is made.
/// The mock assemble endpoint returns a 200 response simulating the case where all chunks
/// are already uploaded.
fn ensure_correct_assemble_call() {
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
            .with_response_body(
                r#"{
                "21b76b717dbbd8c89e42d92b29667ac87aa3c124": {
                    "state": "ok",
                    "missingChunks": []
                }
            }"#,
            ),
        )
        .assert_cmd(
            "debug-files upload --include-sources tests/integration/_fixtures/SrcGenSampleApp.pdb"
                .split(' '),
        )
        .with_default_token()
        .run_and_assert(AssertCommand::Success);
}

#[test]
/// This test simulates a full chunk upload (with only one chunk).
/// It verifies that the Sentry CLI makes the expected API calls to the chunk upload endpoint
/// and that the data sent to the chunk upload endpoint is exactly as expected.
/// It also verifies that the correct calls are made to the assemble endpoint.
fn ensure_correct_chunk_upload() {
    let is_first_assemble_call = AtomicBool::new(true);

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

                    let decompressed = chunk_upload::decompress_chunks(body, boundary)
                        .expect("chunks should be valid gzip data");

                    let expected_content =
                        fs::read("tests/integration/_fixtures/SrcGenSampleApp.pdb")
                            .expect("fixture file should be readable");

                    assert_eq!(decompressed.len(), 1, "expected exactly one chunk");
                    assert!(
                        decompressed.contains(&expected_content),
                        "decompressed chunk should match the source file"
                    );

                    vec![] // Client does not expect a response body
                }),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "POST",
                "/api/0/projects/wat-org/wat-project/files/difs/assemble/",
            )
            .with_header_matcher("content-type", "application/json")
            .with_matcher(r#"{"21b76b717dbbd8c89e42d92b29667ac87aa3c124":{"name":"SrcGenSampleApp.pdb","debug_id":"c02651ae-cd6f-492d-bc33-0b83111e7106-8d8e7c60","chunks":["21b76b717dbbd8c89e42d92b29667ac87aa3c124"]}}"#)
            .with_response_fn(move |_| {
                if is_first_assemble_call.swap(false, Ordering::Relaxed) {
                    r#"{
                        "21b76b717dbbd8c89e42d92b29667ac87aa3c124": {
                            "state": "not_found",
                            "missingChunks": ["21b76b717dbbd8c89e42d92b29667ac87aa3c124"]
                        }
                    }"#
                } else {
                    r#"{
                        "21b76b717dbbd8c89e42d92b29667ac87aa3c124": {
                            "state": "created",
                            "missingChunks": []
                        }
                    }"#
                }
                .into()
            })
            .expect(2),
        )
        .assert_cmd(
            "debug-files upload --include-sources tests/integration/_fixtures/SrcGenSampleApp.pdb"
                .split(' '),
        )
        .with_default_token()
        .run_and_assert(AssertCommand::Success);
}

#[test]
/// This test verifies a correct chunk upload of multiple debug files.
fn chunk_upload_multiple_files() {
    let is_first_assemble_call = AtomicBool::new(true);
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

                    let decompressed = chunk_upload::decompress_chunks(body, boundary)
                        .expect("chunks should be valid gzip data");

                    let fixture_dir = "tests/integration/_fixtures/debug_files/upload/chunk_upload_multiple_files";
                    let expected_files: HashSet<Vec<u8>> = ["fibonacci", "fibonacci-fast", "main"]
                        .iter()
                        .map(|name| fs::read(format!("{fixture_dir}/{name}")).expect("fixture should be readable"))
                        .collect();

                    assert_eq!(decompressed.len(), 3, "expected exactly three chunks");
                    let decompressed: HashSet<_> =
                        decompressed.into_iter().collect();
                    assert_eq!(
                        decompressed, expected_files,
                        "decompressed chunks should match the fixture files"
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
            .with_response_fn(move |_| {
                if is_first_assemble_call.swap(false, Ordering::Relaxed) {
                    r#"{
                        "6e217f035ed538d4d6c14129baad5cb52e680e74": {
                            "state": "not_found",
                            "missingChunks": ["6e217f035ed538d4d6c14129baad5cb52e680e74"]
                        },
                        "500848b7815119669a292f2ae1f44af11d7aa2d3": {
                            "state": "not_found",
                            "missingChunks": ["500848b7815119669a292f2ae1f44af11d7aa2d3"]
                        },
                        "fc27d95861d56fe16a2b66150e31652b76e8c678": {
                            "state": "not_found",
                            "missingChunks": ["fc27d95861d56fe16a2b66150e31652b76e8c678"]
                        }
                    }"#
                } else {
                    r#"{
                        "6e217f035ed538d4d6c14129baad5cb52e680e74": {
                            "state": "created",
                            "missingChunks": []
                        },
                        "500848b7815119669a292f2ae1f44af11d7aa2d3": {
                            "state": "created",
                            "missingChunks": []
                        },
                        "fc27d95861d56fe16a2b66150e31652b76e8c678": {
                            "state": "created",
                            "missingChunks": []
                        }
                    }"#
                }
                .into()
            })
            .expect(2),
        )
        .assert_cmd(vec![
            "debug-files",
            "upload",
            "tests/integration/_fixtures/debug_files/upload/chunk_upload_multiple_files",
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Success);
}

#[test]
/// This test verifies a correct chunk upload of multiple debug files,
/// where one of the files is already uploaded.
/// Only the missing files should be uploaded.
fn chunk_upload_multiple_files_only_some() {
    let is_first_assemble_call = AtomicBool::new(true);
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

                    let decompressed = chunk_upload::decompress_chunks(body, boundary)
                        .expect("chunks should be valid gzip data");

                    let fixture_dir = "tests/integration/_fixtures/debug_files/upload/chunk_upload_multiple_files";
                    // The chunk already on the server, as per the dif/assemble mock below.
                    // If it shows up in the request body, the client is re-uploading a chunk
                    // it was told to skip.
                    const ALREADY_UPLOADED_HASH: &str = "6e217f035ed538d4d6c14129baad5cb52e680e74";
                    let expected_uploads: HashSet<Vec<u8>> = ["fibonacci", "fibonacci-fast", "main"]
                        .into_iter()
                        .map(|name| {
                            fs::read(format!("{fixture_dir}/{name}"))
                                .expect("fixture should be readable")
                        })
                        .filter(|contents| {
                            Sha1::from(contents).digest().to_string() != ALREADY_UPLOADED_HASH
                        })
                        .collect();

                    assert_eq!(
                        HashSet::from_iter(decompressed),
                        expected_uploads,
                        "uploaded chunks should be exactly the two fixtures not already on the server",
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
            .with_response_fn(move |_| {
                if is_first_assemble_call.swap(false, Ordering::Relaxed) {
                    r#"{
                        "6e217f035ed538d4d6c14129baad5cb52e680e74": {
                            "state": "ok",
                            "missingChunks": []
                        },
                        "500848b7815119669a292f2ae1f44af11d7aa2d3": {
                            "state": "not_found",
                            "missingChunks": ["500848b7815119669a292f2ae1f44af11d7aa2d3"]
                        },
                        "fc27d95861d56fe16a2b66150e31652b76e8c678": {
                            "state": "not_found",
                            "missingChunks": ["fc27d95861d56fe16a2b66150e31652b76e8c678"]
                        }
                    }"#
                } else {
                    r#"{
                        "6e217f035ed538d4d6c14129baad5cb52e680e74": {
                            "state": "ok",
                            "missingChunks": []
                        },
                        "500848b7815119669a292f2ae1f44af11d7aa2d3": {
                            "state": "created",
                            "missingChunks": []
                        },
                        "fc27d95861d56fe16a2b66150e31652b76e8c678": {
                            "state": "created",
                            "missingChunks": []
                        }
                    }"#
                }
                .into()
            })
            .expect(2),
        )
        .assert_cmd(vec![
            "debug-files",
            "upload",
            "tests/integration/_fixtures/debug_files/upload/chunk_upload_multiple_files",
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Success);
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SmallChunkUploadOptions {
    chunk_size: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssembleFileResponse {
    missing_chunks: Vec<String>,
}

fn sha1_hex(data: &[u8]) -> String {
    let mut sha = Sha1::new();
    sha.update(data);
    sha.digest().to_string()
}

fn expected_uploaded_chunk_digests(assemble_response_path: &str) -> BTreeMap<String, usize> {
    let chunk_upload_options: SmallChunkUploadOptions = serde_json::from_slice(
        &fs::read("tests/integration/_responses/debug_files/get-chunk-upload-small-chunks.json")
            .expect("chunk upload options response file should be present"),
    )
    .expect("chunk upload options response should be valid JSON");

    let assemble_response: BTreeMap<String, AssembleFileResponse> = serde_json::from_slice(
        &fs::read(assemble_response_path).expect("assemble response file should be present"),
    )
    .expect("assemble response should be valid JSON");

    let fixture_dir = "tests/integration/_fixtures/debug_files/upload/chunk_upload_multiple_files";
    let mut chunk_digests = BTreeMap::new();

    for fixture_name in ["fibonacci", "fibonacci-fast", "main"] {
        let fixture_data = fs::read(format!("{fixture_dir}/{fixture_name}"))
            .expect("fixture file should be readable");
        let file_checksum = sha1_hex(&fixture_data);
        let file_response = assemble_response
            .get(&file_checksum)
            .expect("assemble response should contain an entry for every fixture file");

        for chunk_checksum in fixture_data
            .chunks(chunk_upload_options.chunk_size)
            .map(sha1_hex)
            .filter(|chunk_checksum| file_response.missing_chunks.contains(chunk_checksum))
        {
            *chunk_digests.entry(chunk_checksum).or_default() += 1;
        }
    }

    chunk_digests
}

#[test]
/// This test verifies a correct chunk upload of multiple debug files
/// with a small chunk size (2048 bytes).
/// There are also multiple requests to the chunk upload endpoint, since
/// there are more chunk than the maximum allowed per request.
fn chunk_upload_multiple_files_small_chunks() {
    /// The chunk upload options specify that a single request should contain
    /// at most 64 chunks.
    const CHUNKS_PER_REQUEST: usize = 64;

    let expected_chunk_digests = expected_uploaded_chunk_digests(
        "tests/integration/_responses/debug_files/assemble-chunk-upload-small-chunks.json",
    );
    let expected_total_chunks: usize = expected_chunk_digests.values().sum();
    let expected_requests = expected_total_chunks.div_ceil(CHUNKS_PER_REQUEST);

    let received_chunk_digests = Arc::new(Mutex::new(BTreeMap::<String, usize>::new()));
    let received_chunk_digests_closure = received_chunk_digests.clone();

    let is_first_assemble_call = AtomicBool::new(true);
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/")
                .with_response_file("debug_files/get-chunk-upload-small-chunks.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "POST",
                "/api/0/projects/wat-org/wat-project/files/difs/assemble/",
            )
            .with_header_matcher("content-type", "application/json")
            .with_response_fn(move |_| {
                if is_first_assemble_call.swap(false, Ordering::Relaxed) {
                    fs::read(
                        "tests/integration/_responses/debug_files/\
                         assemble-chunk-upload-small-chunks.json",
                    )
                    .expect("first assemble response file should be present")
                } else {
                    r#"{
                        "6e217f035ed538d4d6c14129baad5cb52e680e74": {
                            "state": "created",
                            "missingChunks": []
                        },
                        "500848b7815119669a292f2ae1f44af11d7aa2d3": {
                            "state": "created",
                            "missingChunks": []
                        },
                        "fc27d95861d56fe16a2b66150e31652b76e8c678": {
                            "state": "created",
                            "missingChunks": []
                        }
                    }"#
                    .into()
                }
            })
            .expect(2),
        )
        .mock_endpoint(
            MockEndpointBuilder::new("POST", "/api/0/organizations/wat-org/chunk-upload/")
                .with_response_fn(move |request| {
                    let boundary = chunk_upload::boundary_from_request(request)
                        .expect("content-type header should be a valid multipart/form-data header");

                    let body = request.body().expect("body should be readable");

                    let decompressed = chunk_upload::decompress_chunks(body, boundary)
                        .expect("chunks should be valid gzip data");

                    // No single request should contain more than CHUNKS_PER_REQUEST chunks.
                    assert!(decompressed.len() <= CHUNKS_PER_REQUEST);

                    let request_chunk_digests = chunk_upload::chunk_digest_counts(&decompressed);
                    let mut received_chunk_digests = received_chunk_digests_closure
                        .lock()
                        .expect("should be able to lock mutex");
                    for (chunk_digest, count) in request_chunk_digests {
                        *received_chunk_digests.entry(chunk_digest).or_default() += count;
                    }

                    vec![]
                })
                .expect(expected_requests),
        )
        .assert_cmd(vec![
            "debug-files",
            "upload",
            "tests/integration/_fixtures/debug_files/upload/chunk_upload_multiple_files",
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Success);

    let received_chunk_digests = received_chunk_digests
        .lock()
        .expect("should be able to lock mutex");
    assert_eq!(
        *received_chunk_digests, expected_chunk_digests,
        "uploaded chunk digests should match the missing chunks requested by assemble"
    );
}

#[test]
/// This test is similar to `chunk_upload_multiple_files_small_chunks`, but
/// here, only some of the chunks are missing.
/// Of the three files two be uploaded, one is missing all chunks, and two
/// are missing only some (including one file that is missing only one chunk).
/// We verify that only the missing chunks get uploaded.
fn chunk_upload_multiple_files_small_chunks_only_some() {
    /// The chunk upload options specify that a single request should contain
    /// at most 64 chunks.
    const CHUNKS_PER_REQUEST: usize = 64;

    let expected_chunk_digests = expected_uploaded_chunk_digests(
        "tests/integration/_responses/debug_files/assemble-chunk-upload-small-chunks-only-some-missing.json",
    );
    let expected_total_chunks: usize = expected_chunk_digests.values().sum();
    let expected_requests = expected_total_chunks.div_ceil(CHUNKS_PER_REQUEST);

    let received_chunk_digests = Arc::new(Mutex::new(BTreeMap::<String, usize>::new()));
    let received_chunk_digests_closure = received_chunk_digests.clone();

    let is_first_assemble_call = AtomicBool::new(true);

    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/")
                .with_response_file("debug_files/get-chunk-upload-small-chunks.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "POST",
                "/api/0/projects/wat-org/wat-project/files/difs/assemble/",
            )
            .with_header_matcher("content-type", "application/json")
            .with_response_fn(move |_| {
                if is_first_assemble_call.swap(false, Ordering::Relaxed) {
                    fs::read(
                        "tests/integration/_responses/debug_files/\
                         assemble-chunk-upload-small-chunks-only-some-missing.json",
                    )
                    .expect("first assemble response file should be present")
                } else {
                    r#"{
                        "6e217f035ed538d4d6c14129baad5cb52e680e74": {
                            "state": "created",
                            "missingChunks": []
                        },
                        "500848b7815119669a292f2ae1f44af11d7aa2d3": {
                            "state": "created",
                            "missingChunks": []
                        },
                        "fc27d95861d56fe16a2b66150e31652b76e8c678": {
                            "state": "created",
                            "missingChunks": []
                        }
                    }"#
                    .into()
                }
            })
            .expect(2),
        )
        .mock_endpoint(
            MockEndpointBuilder::new("POST", "/api/0/organizations/wat-org/chunk-upload/")
                .with_response_fn(move |request| {
                    let boundary = chunk_upload::boundary_from_request(request)
                        .expect("content-type header should be a valid multipart/form-data header");

                    let body = request.body().expect("body should be readable");

                    let decompressed = chunk_upload::decompress_chunks(body, boundary)
                        .expect("chunks should be valid gzip data");

                    // No single request should contain more than CHUNKS_PER_REQUEST chunks.
                    assert!(decompressed.len() <= CHUNKS_PER_REQUEST);

                    let request_chunk_digests = chunk_upload::chunk_digest_counts(&decompressed);
                    let mut received_chunk_digests = received_chunk_digests_closure
                        .lock()
                        .expect("should be able to lock mutex");
                    for (chunk_digest, count) in request_chunk_digests {
                        *received_chunk_digests.entry(chunk_digest).or_default() += count;
                    }

                    vec![]
                })
                .expect(expected_requests),
        )
        .assert_cmd(vec![
            "debug-files",
            "upload",
            "tests/integration/_fixtures/debug_files/upload/chunk_upload_multiple_files",
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Success);

    let received_chunk_digests = received_chunk_digests
        .lock()
        .expect("should be able to lock mutex");
    assert_eq!(
        *received_chunk_digests, expected_chunk_digests,
        "uploaded chunk digests should match the missing chunks requested by assemble"
    );
}

#[test]
fn test_dif_too_big() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/")
                .with_response_file("debug_files/get-chunk-upload-small-max-size.json"),
        )
        .assert_cmd(vec![
            "debug-files",
            "upload",
            "tests/integration/_fixtures/SrcGenSampleApp.pdb",
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Failure);
}
