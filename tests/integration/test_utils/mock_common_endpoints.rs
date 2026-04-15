use std::{
    fmt::Display,
    sync::atomic::{AtomicBool, Ordering},
};

use serde_json::json;

use crate::integration::test_utils::MockEndpointBuilder;

/// Returns an iterator over builders for the common upload endpoints.
/// These can be used to generate mocks for the upload endpoints.
pub(super) fn common_upload_endpoints(
    server_url: impl Display,
    chunk_size: Option<usize>,
    simulate_missing_chunks: bool,
) -> impl Iterator<Item = MockEndpointBuilder> {
    let chunk_size = chunk_size.unwrap_or(8388608);
    let assemble_endpoint = "/api/0/organizations/wat-org/artifactbundle/assemble/";
    let chunk_upload_response = format!(
        "{{
            \"url\": \"{server_url}/api/0/organizations/wat-org/chunk-upload/\",
            \"chunkSize\": {chunk_size},
            \"chunksPerRequest\": 64,
            \"maxRequestSize\": 33554432,
            \"concurrency\": 8,
            \"hashAlgorithm\": \"sha1\",
            \"accept\": []
          }}",
    );

    let is_first_request = AtomicBool::new(true);

    vec![
        MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/")
            .with_response_body(chunk_upload_response),
        MockEndpointBuilder::new("POST", "/api/0/organizations/wat-org/chunk-upload/")
            .with_response_body("[]"),
        MockEndpointBuilder::new("POST", assemble_endpoint)
            .with_response_fn(move |request| {
                let body = request.body().expect("body should be readable");
                let response =
                    if is_first_request.swap(false, Ordering::Relaxed) && simulate_missing_chunks {
                        // On the first request, report all chunks from the request body
                        // as missing so the CLI uploads them.
                        let parsed: serde_json::Value = serde_json::from_slice(body)
                            .expect("assemble body should be valid JSON");
                        let chunks = parsed
                            .get("chunks")
                            .and_then(|c| c.as_array())
                            .cloned()
                            .unwrap_or_default();
                        json!({
                            "state": "not_found",
                            "missingChunks": chunks,
                        })
                    } else {
                        json!({
                            "state": "created",
                            "missingChunks": [],
                        })
                    };

                serde_json::to_vec(&response).expect("failed to serialize response")
            })
            .expect_at_least(1),
    ]
    .into_iter()
}
