mod bash_hook;
mod debug_files;
mod deploys;
mod events;
mod help;
mod info;
mod login;
mod monitors;
mod organizations;
mod projects;
mod releases;
mod send_envelope;
mod send_event;
mod sourcemaps;
mod uninstall;
mod update;
mod upload_dif;
mod upload_proguard;

use mockito::{mock, server_url, Matcher, Mock};
use trycmd::TestCases;

pub const UTC_DATE_FORMAT: &str = r#"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}.\d{6,9}Z"#;
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
        .case(format!("tests/integration/_cases/{}", path));
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
        self.response_file = Some(format!("tests/integration/_responses/{}", path));
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
