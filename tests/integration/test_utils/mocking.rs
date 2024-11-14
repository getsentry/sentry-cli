use mockito::{Matcher, Mock};

pub struct EndpointOptions {
    method: String,
    endpoint: String,
    status: usize,
    response_body: Option<String>,
    response_file: Option<String>,
    matcher: Option<Matcher>,
    header_matcher: Option<(&'static str, Matcher)>,
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
            header_matcher: None,
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

    /// Matches a header of the mock endpoint. The header must be present and its value must
    /// match the provided matcher in order for the endpoint to be reached.
    pub fn with_header_matcher(mut self, key: &'static str, matcher: Matcher) -> Self {
        self.header_matcher = Some((key, matcher));
        self
    }
}

pub fn mock_endpoint(opts: EndpointOptions) -> Mock {
    let mut mock = mockito::mock(opts.method.as_str(), opts.endpoint.as_str())
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

    if let Some((key, matcher)) = opts.header_matcher {
        mock = mock.match_header(key, matcher);
    }

    mock.create()
}
