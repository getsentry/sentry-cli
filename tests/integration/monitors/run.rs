use crate::integration::{mock_endpoint, register_test, EndpointOptions};

#[test]
fn command_monitors_run() {
    if cfg!(windows) {
        register_test("monitors/monitors-run-win.trycmd");
    } else {
        register_test("monitors/monitors-run.trycmd");
    }
}

#[test]
fn command_monitors_run_token_auth() {
    let _server = mock_endpoint(
        EndpointOptions::new("POST", "/api/0/monitors/foo-monitor/checkins/", 200)
            .with_response_file("monitors/post-monitors.json"),
    );
    if cfg!(windows) {
        register_test("monitors/monitors-run-token-auth-win.trycmd").env("SENTRY_DSN", "");
    } else {
        register_test("monitors/monitors-run-token-auth.trycmd").env("SENTRY_DSN", "");
    }
}

#[test]
fn command_monitors_run_osenv() {
    register_test("monitors/monitors-run-osenv.trycmd");
}

#[test]
fn command_monitors_run_environment() {
    register_test("monitors/monitors-run-environment.trycmd");
}

#[test]
fn command_monitors_run_help() {
    register_test("monitors/monitors-run-help.trycmd");
}
