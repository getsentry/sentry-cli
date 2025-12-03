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
    initial_missing_chunks: Option<Vec<&'static str>>,
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
            .with_response_fn(move |_| {
                let response = if let Some(missing_chunks) = is_first_request
                    .swap(false, Ordering::Relaxed)
                    .then_some(initial_missing_chunks.as_ref())
                    .flatten()
                {
                    json!({
                        "state": "not_found",
                        "missingChunks": missing_chunks,
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
