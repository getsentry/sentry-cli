use crate::integration::{
    mock_endpoint, register_test, register_test_with_opts, AuthMode, EndpointOptions,
    RegisterOptions,
};

#[test]
fn command_monitors_run() {
    // TODO: How do I turn off token auth and force DSN auth in the tests?

    let _server = mock_endpoint(
        EndpointOptions::new("POST", "/api/0/monitors/foo-monitor/checkins/", 200)
            .with_response_file("monitors/post-monitors.json"),
    );

    let opts = RegisterOptions {
        auth_mode: AuthMode::Dsn,
    };

    if cfg!(windows) {
        register_test_with_opts("monitors/monitors-run-win.trycmd", opts);
    } else {
        register_test_with_opts("monitors/monitors-run.trycmd", opts);
    }
}

#[test]
fn command_monitors_run_token_auth() {
    let _server = mock_endpoint(
        EndpointOptions::new("POST", "/api/0/monitors/foo-monitor/checkins/", 200)
            .with_response_file("monitors/post-monitors.json"),
    );
    if cfg!(windows) {
        register_test("monitors/monitors-run-token-auth-win.trycmd");
    } else {
        register_test("monitors/monitors-run-token-auth.trycmd");
    }
}

#[test]
fn command_monitors_run_env() {
    // TODO: How do I turn off token auth and force DSN auth in the tests?

    let _server = mock_endpoint(
        EndpointOptions::new("POST", "/api/0/monitors/foo-monitor/checkins/", 200)
            .with_response_file("monitors/post-monitors.json"),
    );
    register_test_with_opts(
        "monitors/monitors-run-env.trycmd",
        RegisterOptions {
            auth_mode: AuthMode::Dsn,
        },
    );
}

#[test]
fn command_monitors_run_help() {
    register_test("monitors/monitors-run-help.trycmd");
}
