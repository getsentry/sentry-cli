use std::sync::atomic::{AtomicU8, Ordering};

use crate::integration::test_utils::AssertCommand;
use crate::integration::{MockEndpointBuilder, TestManager};

/// A test org auth token with org="wat-org" and empty URL.
/// Format: sntrys_{base64_payload}_{base64_secret}
/// Payload: {"iat":1704374159.069583,"url":"","region_url":"","org":"wat-org"}
const ORG_AUTH_TOKEN_WAT_ORG: &str = "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";

#[test]
fn command_upload_dart_symbol_map_missing_capability() {
    // Server does not advertise `dartsymbolmap` capability → command should bail early.
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/")
                .with_response_file("debug_files/get-chunk-upload.json"),
        )
        .assert_cmd([
            "dart-symbol-map",
            "upload",
            "tests/integration/_fixtures/dart_symbol_map/dartsymbolmap.json",
            // Use a fixture with a single Debug ID
            "tests/integration/_fixtures/Sentry.Samples.Console.Basic.pdb",
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Failure);
}

#[test]
fn command_upload_dart_symbol_map_chunk_upload_flow() {
    // Happy path: server supports dartsymbolmap capability, file needs upload, then assembles to ok.
    let call_count = AtomicU8::new(0);

    TestManager::new()
        // Server advertises capability including `dartsymbolmap`.
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/")
                .with_response_file("dart_symbol_map/get-chunk-upload.json"),
        )
        // Accept chunk upload requests for the missing chunks; no validation needed here.
        .mock_endpoint(MockEndpointBuilder::new(
            "POST",
            "/api/0/organizations/wat-org/chunk-upload/",
        ))
        // Assemble flow: 1) not_found (missingChunks), 2) created, 3) ok
        .mock_endpoint(
            MockEndpointBuilder::new(
                "POST",
                "/api/0/projects/wat-org/wat-project/files/difs/assemble/",
            )
            .with_header_matcher("content-type", "application/json")
            .with_response_fn(move |request| {
                let body = request.body().expect("body should be readable");
                let body_json: serde_json::Value = serde_json::from_slice(body)
                    .expect("request body should be valid JSON");

                // The request map has a single entry keyed by checksum; reuse it in responses.
                let (checksum, _obj) = body_json
                    .as_object()
                    .and_then(|m| m.iter().next())
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .expect("assemble request must contain at least one object");

                match call_count.fetch_add(1, Ordering::Relaxed) {
                    0 => format!(
                        "{{\"{checksum}\":{{\"state\":\"not_found\",\"missingChunks\":[\"{checksum}\"]}}}}"
                    )
                    .into(),
                    1 => format!(
                        "{{\"{checksum}\":{{\"state\":\"created\",\"missingChunks\":[]}}}}"
                    )
                    .into(),
                    2 => format!(
                        "{{\"{checksum}\":{{\"state\":\"ok\",\"detail\":null,\"missingChunks\":[],\"dif\":{{\"id\":\"1\",\"uuid\":\"00000000-0000-0000-0000-000000000000\",\"debugId\":\"00000000-0000-0000-0000-000000000000\",\"objectName\":\"dartsymbolmap.json\",\"cpuName\":\"any\",\"headers\":{{\"Content-Type\":\"application/octet-stream\"}},\"size\":1,\"sha1\":\"{checksum}\",\"dateCreated\":\"1776-07-04T12:00:00.000Z\",\"data\":{{}}}}}}}}"
                    )
                    .into(),
                    n => panic!(
                        "Only 3 calls to the assemble endpoint expected, but there were {}.",
                        n + 1
                    ),
                }
            })
            .expect(3),
        )
        .assert_cmd([
            "dart-symbol-map",
            "upload",
            "tests/integration/_fixtures/dart_symbol_map/dartsymbolmap.json",
            // Use a fixture with a single Debug ID (embedded PDB)
            "tests/integration/_fixtures/Sentry.Samples.Console.Basic.pdb",
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Success);
}

#[test]
fn command_upload_dart_symbol_map_invalid_mapping() {
    // Invalid mapping (odd number of entries) should fail before any HTTP calls.
    TestManager::new()
        .assert_cmd([
            "dart-symbol-map",
            "upload",
            "tests/integration/_fixtures/dart_symbol_map/dartsymbolmap-invalid.json",
            "tests/integration/_fixtures/Sentry.Samples.Console.Basic.pdb",
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Failure);
}

#[test]
fn command_upload_dart_symbol_map_org_from_token() {
    // When no --org is provided and SENTRY_ORG is not set, the org should be resolved
    // from the org auth token. This test verifies the fix for CLI-260.
    //
    // The test uses an org auth token with org="wat-org" (matching mock server paths).
    // By unsetting SENTRY_ORG and not providing --org, we verify the org is extracted
    // from the token.
    let call_count = AtomicU8::new(0);

    TestManager::new()
        // Server advertises capability including `dartsymbolmap`.
        // This endpoint uses "wat-org" in the path - if org resolution fails,
        // the request would go to a different path and not match.
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/")
                .with_response_file("dart_symbol_map/get-chunk-upload.json"),
        )
        // Accept chunk upload requests for the missing chunks
        .mock_endpoint(MockEndpointBuilder::new(
            "POST",
            "/api/0/organizations/wat-org/chunk-upload/",
        ))
        // Assemble flow: 1) not_found (missingChunks), 2) created, 3) ok
        .mock_endpoint(
            MockEndpointBuilder::new(
                "POST",
                "/api/0/projects/wat-org/wat-project/files/difs/assemble/",
            )
            .with_header_matcher("content-type", "application/json")
            .with_response_fn(move |request| {
                let body = request.body().expect("body should be readable");
                let body_json: serde_json::Value =
                    serde_json::from_slice(body).expect("request body should be valid JSON");

                let (checksum, _obj) = body_json
                    .as_object()
                    .and_then(|m| m.iter().next())
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .expect("assemble request must contain at least one object");

                match call_count.fetch_add(1, Ordering::Relaxed) {
                    0 => format!(
                        "{{\"{checksum}\":{{\"state\":\"not_found\",\"missingChunks\":[\"{checksum}\"]}}}}"
                    )
                    .into(),
                    1 => format!(
                        "{{\"{checksum}\":{{\"state\":\"created\",\"missingChunks\":[]}}}}"
                    )
                    .into(),
                    2 => format!(
                        "{{\"{checksum}\":{{\"state\":\"ok\",\"detail\":null,\"missingChunks\":[],\"dif\":{{\"id\":\"1\",\"uuid\":\"00000000-0000-0000-0000-000000000000\",\"debugId\":\"00000000-0000-0000-0000-000000000000\",\"objectName\":\"dartsymbolmap.json\",\"cpuName\":\"any\",\"headers\":{{\"Content-Type\":\"application/octet-stream\"}},\"size\":1,\"sha1\":\"{checksum}\",\"dateCreated\":\"1776-07-04T12:00:00.000Z\",\"data\":{{}}}}}}}}"
                    )
                    .into(),
                    n => panic!(
                        "Only 3 calls to the assemble endpoint expected, but there were {}.",
                        n + 1
                    ),
                }
            })
            .expect(3),
        )
        .assert_cmd([
            "dart-symbol-map",
            "upload",
            // No --org flag provided!
            "tests/integration/_fixtures/dart_symbol_map/dartsymbolmap.json",
            "tests/integration/_fixtures/Sentry.Samples.Console.Basic.pdb",
        ])
        // Use org auth token with embedded org="wat-org" instead of default token
        .env("SENTRY_AUTH_TOKEN", ORG_AUTH_TOKEN_WAT_ORG)
        // Explicitly unset SENTRY_ORG to ensure org comes from token
        .env("SENTRY_ORG", "")
        .run_and_assert(AssertCommand::Success);
}
