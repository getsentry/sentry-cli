use std::ffi::OsStr;
use std::fmt::Display;

use assert_cmd::Command;
use mockito::{Mock, Server, ServerGuard};
use thiserror::Error;
use trycmd::TestCases;

use crate::integration::{env, MockEndpointBuilder, VERSION};

use super::{mock_common_endpoints, MockServerInfo};

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
        chunk_size: Option<usize>,
        initial_missing_chunks: Option<Vec<&'static str>>,
    ) -> Self {
        mock_common_endpoints::common_upload_endpoints(
            self.server_url(),
            chunk_size,
            initial_missing_chunks,
        )
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

    /// Define an assert_cmd test.
    /// The args contain the command line arguments which will be passed to `sentry-cli`.
    /// The test is run when the appropriate function is called on the returned
    /// `AssertCmdTestManager`.
    /// The test manager handles setting the environment variables for the test.
    pub fn assert_cmd<I, S>(self, args: I) -> AssertCmdTestManager
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        AssertCmdTestManager::new(self, args)
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

pub struct AssertCmdTestManager {
    manager: TestManager,
    command: Command,
}

/// The type of assertion to perform on the command result.
// Currently we only assert success, but we may add other assertions
// (e.g. failure or skip) in the future.
pub enum AssertCommand {
    /// Assert that the command succeeds (i.e. returns a `0` exit code).
    Success,
    /// Assert that the command fails (i.e. returns a non-zero exit code).
    Failure,
}

impl AssertCmdTestManager {
    /// Set the auth token environment variable to a fake value.
    /// This may be needed when running a Sentry CLI command that checks that
    /// an auth token is set. No token is set by default.
    pub fn with_default_token(mut self) -> Self {
        env::set_auth_token(|k, v| {
            self.command.env(k, v.as_ref());
        });

        self
    }

    /// Set a custom environment variable for the test.
    pub fn env(
        mut self,
        key: impl AsRef<std::ffi::OsStr>,
        value: impl AsRef<std::ffi::OsStr>,
    ) -> Self {
        self.command.env(key, value);
        self
    }

    /// Run the command and perform assertions.
    ///
    /// This function asserts both the mocks and the command result.
    /// The mocks are asserted first, since a failure in the mocks
    /// could cause the command to fail. The function consumes the
    /// `AssertCmdTestManager`, as it should not be used after this call.
    ///
    /// Panics if any assertions fail.
    pub fn run_and_assert(mut self, assert: AssertCommand) {
        let command_result = self.command.assert();
        self.manager.assert_mock_endpoints();

        match assert {
            AssertCommand::Success => command_result.success(),
            AssertCommand::Failure => command_result.failure(),
        };
    }

    fn new<I, S>(manager: TestManager, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = Command::cargo_bin("sentry-cli").expect("sentry-cli should be available");
        command.args(args);

        env::set(manager.server_info(), |k, v| {
            command.env(k, v.as_ref());
        });

        Self { manager, command }
    }
}
