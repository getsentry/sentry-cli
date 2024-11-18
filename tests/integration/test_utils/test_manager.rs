use std::fmt::Display;

use mockito::{Mock, Server, ServerGuard};
use thiserror::Error;
use trycmd::TestCases;

use crate::integration::{env, MockEndpointBuilder, VERSION};

use super::{mock_common_endpoints, ChunkOptions, MockServerInfo, ServerBehavior};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to insert server variable")]
    InsertServerVar(#[from] trycmd::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Helper struct for managing integration tests.
/// Allows for mocking endpoints and registering different types of tests.
pub struct TestManager {
    mocks: Vec<Mock>,
    server: ServerGuard,
}

impl TestManager {
    /// Create a new `TestManager`.
    /// The test manager has no mocked endpoints by default.
    pub fn new() -> Self {
        Self {
            mocks: vec![],
            server: Server::new(),
        }
    }

    /// Create a mock endpoint on the mockito test server with the given options.
    /// Returns the updated `TestManager` with the new mock endpoint.
    pub fn mock_endpoint(mut self, opts: MockEndpointBuilder) -> Self {
        self.mocks.push(opts.create(&mut self.server));
        self
    }

    /// Mock the common upload endpoints.
    pub fn mock_common_upload_endpoints(
        self,
        behavior: ServerBehavior,
        chunk_options: ChunkOptions,
    ) -> Self {
        mock_common_endpoints::common_upload_endpoints(self.server_url(), behavior, chunk_options)
            .fold(self, |manager, builder| manager.mock_endpoint(builder))
    }

    /// Assert that all mocks have been called the correct number of times.
    ///
    /// This method consumes the `TestManager`, thereby also stopping the mocks from being
    /// served, since after asserting the mocks, they should not be needed anymore.
    pub fn assert_mock_endpoints(self) {
        for mock in self.mocks {
            mock.assert();
        }
    }

    /// Register a trycmd test.
    /// The test is run when the returned `TrycmdTestManager` is dropped.
    /// Further configuration can be done with the `TrycmdTestManager`.
    pub fn register_trycmd_test(self, path: impl Display) -> TrycmdTestManager {
        TrycmdTestManager::new(self, path)
    }

    /// Get the URL of the mock server.
    pub fn server_url(&self) -> String {
        self.server().url()
    }

    /// Get information about mock server, needed for setting environment variables.
    pub fn server_info(&self) -> MockServerInfo {
        self.server().into()
    }

    /// Get reference to the mockito server.
    fn server(&self) -> &ServerGuard {
        &self.server
    }
}

/// Helper struct for managing trycmd tests.
/// The tests are run when the `TrycmdTestManager` is dropped.
pub struct TrycmdTestManager {
    // The ordering of the fields is important here, since we need to
    // ensure that `manager` is dropped AFTER `test_case`.
    //
    // This is because the mock server is only alive until `manager`
    // is dropped, and the mock server is required for the test case
    // to run.  Trycmd tests run when the `TestCases` are dropped, so
    // we must ensure that `manager` is dropped after `test_case`.
    //
    // The Rust language specifies that fields are dropped in the order
    // of their declaration (https://doc.rust-lang.org/reference/destructors.html).
    //
    // So, `test_case` MUST be declared BEFORE `manager`.
    test_case: TestCases,
    manager: TestManager,
}

impl TrycmdTestManager {
    /// Set a custom environment variable for the test.
    pub fn env(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.test_case.env(key, value);
        self
    }

    /// Register an additional trycmd test case on this test manager.
    /// The path is relative to `tests/integration/_cases/`.
    pub fn register_trycmd_test(self, path: impl Display) -> Self {
        self.test_case
            .case(format!("tests/integration/_cases/{path}"));
        self
    }

    /// Set the auth token environment variable to a fake value.
    /// This may be needed when running a Sentry CLI command that checks that
    /// an auth token is set. No token is set by default.
    pub fn with_default_token(self) -> Self {
        env::set_auth_token(|k, v| {
            self.test_case.env(k, v);
        });

        self
    }

    /// Insert the server variable into the test case.
    pub fn with_server_var(self) -> Result<Self> {
        self.test_case.insert_var("[SERVER]", self.server().url())?;
        Ok(self)
    }

    /// Assert that all mocks have been called the correct number of times.
    ///
    /// This method also runs the trycmd tests, and consumes the manager.
    pub fn assert_mock_endpoints(self) {
        self.test_case.run();
        self.manager.assert_mock_endpoints();
    }

    fn new(manager: TestManager, path: impl Display) -> Self {
        let test_case = TestCases::new();

        env::set(manager.server_info(), |k, v| {
            test_case.env(k, v);
        });

        test_case.insert_var("[VERSION]", VERSION).unwrap();

        Self { manager, test_case }.register_trycmd_test(path)
    }

    fn server(&self) -> &ServerGuard {
        self.manager.server()
    }
}
