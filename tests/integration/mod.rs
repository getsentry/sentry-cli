mod bash_hook;
mod debug_files;
mod deploys;
mod events;
mod help;
mod info;
mod issues;
mod login;
mod monitors;
mod org_tokens;
mod organizations;
mod projects;
mod react_native;
mod releases;
mod send_envelope;
mod send_event;
mod sourcemaps;
mod uninstall;
mod update;
mod upload_dif;
mod upload_proguard;

use std::fs;
use std::io;
use std::path::Path;

use mockito::{mock, server_url, Matcher, Mock};
use trycmd::TestCases;

pub const UTC_DATE_FORMAT: &str = r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}.\d{6,9}Z";
const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn register_test(path: &str) -> TestCases {
    let test_case = TestCases::new();
    test_case
        .env("SENTRY_INTEGRATION_TEST", "1")
        .env("SENTRY_DUMP_RESPONSES", "dump") // reused default directory of `trycmd` output dumps
        .env("SENTRY_URL", server_url())
        .env("SENTRY_AUTH_TOKEN", "lolnope")
        .env("SENTRY_ORG", "wat-org")
        .env("SENTRY_PROJECT", "wat-project")
        .env("SENTRY_DSN", format!("https://test@{}/1337", server_url()))
        .case(format!("tests/integration/_cases/{path}"));
    test_case.insert_var("[VERSION]", VERSION).unwrap();
    test_case
}
pub struct EndpointOptions {
    pub method: String,
    pub endpoint: String,
    pub status: usize,
    pub response_body: Option<String>,
    pub response_file: Option<String>,
    pub matcher: Option<Matcher>,
}

impl EndpointOptions {
    pub fn new(method: &str, endpoint: &str, status: usize) -> Self {
        EndpointOptions {
            method: method.to_owned(),
            endpoint: endpoint.to_owned(),
            status,
            response_body: None,
            response_file: None,
            matcher: None,
        }
    }

    pub fn with_response_body<T>(mut self, body: T) -> Self
    where
        T: Into<String>,
    {
        self.response_body = Some(body.into());
        self
    }

    pub fn with_response_file(mut self, path: &str) -> Self {
        self.response_file = Some(format!("tests/integration/_responses/{path}"));
        self
    }

    pub fn with_matcher(mut self, matcher: Matcher) -> Self {
        self.matcher = Some(matcher);
        self
    }
}

pub fn mock_endpoint(opts: EndpointOptions) -> Mock {
    let mut mock = mock(opts.method.as_str(), opts.endpoint.as_str())
        .with_status(opts.status)
        .with_header("content-type", "application/json");

    if let Some(response_body) = opts.response_body {
        mock = mock.with_body(response_body);
    }

    if let Some(response_file) = opts.response_file {
        mock = mock.with_body_from_file(response_file);
    }

    if let Some(matcher) = opts.matcher {
        mock = mock.match_body(matcher);
    }

    mock.create()
}

/// Copy files from source to destination recursively.
pub fn copy_recursively(source: impl AsRef<Path>, destination: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let filetype = entry.file_type()?;
        if filetype.is_dir() {
            copy_recursively(entry.path(), destination.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), destination.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub enum ServerBehavior {
    Legacy,
    Modern,
    ModernV2,
}

#[derive(Debug)]
pub struct ChunkOptions {
    chunk_size: usize,
    missing_chunks: Vec<String>,
}

impl Default for ChunkOptions {
    fn default() -> Self {
        Self {
            chunk_size: 8388608,
            missing_chunks: vec![],
        }
    }
}

// Endpoints need to be bound, as they need to live long enough for test to finish
pub fn mock_common_upload_endpoints(
    behavior: ServerBehavior,
    chunk_options: ChunkOptions,
) -> Vec<Mock> {
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
            \"url\": \"{}/api/0/organizations/wat-org/chunk-upload/\",
            \"chunkSize\": {chunk_size},
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
            EndpointOptions::new("POST", assemble_endpoint, 200).with_response_body(format!(
                r#"{{"state":"created","missingChunks":{}}}"#,
                serde_json::to_string(&missing_chunks).unwrap()
            )),
        )
        .expect_at_least(1),
    ]
}

pub fn assert_endpoints(mocks: &[Mock]) {
    for mock in mocks {
        mock.assert();
    }
}
