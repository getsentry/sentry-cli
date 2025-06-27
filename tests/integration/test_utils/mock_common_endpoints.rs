use std::fmt::Display;

use crate::integration::test_utils::MockEndpointBuilder;

/// Returns an iterator over builders for the common upload endpoints.
/// These can be used to generate mocks for the upload endpoints.
pub(super) fn common_upload_endpoints(
    server_url: impl Display,
    behavior: ServerBehavior,
    chunk_options: ChunkOptions,
) -> impl Iterator<Item = MockEndpointBuilder> {
    let ChunkOptions {
        chunk_size,
        missing_chunks,
    } = chunk_options;
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
        ServerBehavior::ModernV2 => (
            "\"release_files\", \"artifact_bundles_v2\"",
            0,
            "/api/0/organizations/wat-org/artifactbundle/assemble/",
        ),
    };
    let chunk_upload_response = format!(
        "{{
            \"url\": \"{server_url}/api/0/organizations/wat-org/chunk-upload/\",
            \"chunkSize\": {chunk_size},
            \"chunksPerRequest\": 64,
            \"maxRequestSize\": 33554432,
            \"concurrency\": 8,
            \"hashAlgorithm\": \"sha1\",
            \"accept\": [{accept}]
          }}",
    );

    vec![
        MockEndpointBuilder::new("POST", "/api/0/projects/wat-org/wat-project/releases/")
            .with_status(208)
            .with_response_file("releases/get-release.json")
            .expect(release_request_count),
        MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/")
            .with_response_body(chunk_upload_response),
        MockEndpointBuilder::new("POST", "/api/0/organizations/wat-org/chunk-upload/")
            .with_response_body("[]"),
        MockEndpointBuilder::new("POST", assemble_endpoint)
            .with_response_body(format!(
                r#"{{"state":"created","missingChunks":{}}}"#,
                serde_json::to_string(&missing_chunks).unwrap()
            ))
            .expect_at_least(1),
    ]
    .into_iter()
}

pub enum ServerBehavior {
    Legacy,
    Modern,
    ModernV2,
}

#[derive(Debug)]
pub struct ChunkOptions {
    pub chunk_size: usize,
    pub missing_chunks: Vec<String>,
}

impl Default for ChunkOptions {
    fn default() -> Self {
        Self {
            chunk_size: 8388608,
            missing_chunks: vec![],
        }
    }
}
