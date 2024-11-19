use mockito::{IntoHeaderName, Matcher, Mock, ServerGuard};

/// Builder for a mock endpoint.
///
/// This struct allows for configuring a mock endpoint to be constructed in
/// the `mock_endpoint()` function. Options can be chained together to create
/// complex mocks.
///
/// The mock is only created once `mock_endpoint()` is called with the builder.
pub struct MockEndpointBuilder {
    /// Function which takes a mockito::ServerGuard and builds the configured mock
    /// on that server.
    builder: Box<dyn FnOnce(&mut ServerGuard) -> Mock>,
}

impl MockEndpointBuilder {
    /// Create a new endpoint options struct
    pub fn new(method: &'static str, endpoint: &'static str) -> Self {
        Self {
            builder: Box::new(move |server| {
                server
                    .mock(method, endpoint)
                    .with_header("content-type", "application/json")
            }),
        }
    }

    /// Set the status code of the mock endpoint.
    /// The default status code (if this method is not called) is 200.
    pub fn with_status(mut self, status: usize) -> Self {
        self.builder = Box::new(move |server| (self.builder)(server).with_status(status));
        self
    }

    /// Set the response body of the mock endpoint.
    pub fn with_response_body<T>(mut self, body: T) -> Self
    where
        T: AsRef<[u8]> + 'static,
    {
        self.builder = Box::new(|server| (self.builder)(server).with_body(body));
        self
    }

    /// Set the response body of the mock endpoint to a file with the given path.
    /// The path is relative to the `tests/integration/_responses` directory.
    pub fn with_response_file(mut self, path: &str) -> Self {
        let response_file = format!("tests/integration/_responses/{path}");

        self.builder = Box::new(|server| (self.builder)(server).with_body_from_file(response_file));
        self
    }

    /// Set the matcher for the response body of the mock endpoint. The mock will only
    /// respond to requests if the response body matches the matcher.
    pub fn with_matcher(mut self, matcher: impl Into<Matcher>) -> Self {
        let matcher = matcher.into();
        self.builder = Box::new(|server| (self.builder)(server).match_body(matcher));
        self
    }

    /// Matches a header of the mock endpoint. The header must be present and its value must
    /// match the provided matcher in order for the endpoint to be reached.
    pub fn with_header_matcher(
        mut self,
        key: impl IntoHeaderName,
        matcher: impl Into<Matcher>,
    ) -> Self {
        let key = key.into_header_name();
        let matcher = matcher.into();
        self.builder = Box::new(move |server| (self.builder)(server).match_header(key, matcher));
        self
    }

    /// Expect the mock endpoint to be hit at least `hits` times.
    ///
    /// This expectation is only checked when the created mock is asserted.
    pub fn expect_at_least(mut self, hits: usize) -> Self {
        self.builder = Box::new(move |server| (self.builder)(server).expect_at_least(hits));
        self
    }

    /// Expect the mock endpoint to be hit exactly `hits` times.
    ///
    /// This expectation is only checked when the created mock is asserted.
    pub fn expect(mut self, hits: usize) -> Self {
        self.builder = Box::new(move |server| (self.builder)(server).expect(hits));
        self
    }

    /// Create and return the mock endpoint on the given server.
    pub(super) fn create(self, server: &mut ServerGuard) -> Mock {
        (self.builder)(server).create()
    }
}
