use crate::integration::{self, EndpointOptions};

#[test]
fn command_monitors_run() {
    let _server =
        integration::mock_endpoint(EndpointOptions::new("POST", "/api/1337/envelope/", 200));
    if cfg!(windows) {
        integration::register_test("monitors/monitors-run-win.trycmd");
    } else {
        integration::register_test("monitors/monitors-run.trycmd");
    }
}

#[test]
fn command_monitors_run_token_auth() {
    let _server = integration::mock_endpoint(
        EndpointOptions::new("POST", "/api/0/monitors/foo-monitor/checkins/", 200)
            .with_response_file("monitors/post-monitors.json"),
    );
    if cfg!(windows) {
        integration::register_test("monitors/monitors-run-token-auth-win.trycmd")
            .env("SENTRY_DSN", "");
    } else {
        integration::register_test("monitors/monitors-run-token-auth.trycmd").env("SENTRY_DSN", "");
    }
}

#[test]
fn command_monitors_run_osenv() {
    let _server =
        integration::mock_endpoint(EndpointOptions::new("POST", "/api/1337/envelope/", 200));
    integration::register_test("monitors/monitors-run-osenv.trycmd");
}

#[test]
fn command_monitors_run_environment() {
    let _server =
        integration::mock_endpoint(EndpointOptions::new("POST", "/api/1337/envelope/", 200));
    integration::register_test("monitors/monitors-run-environment.trycmd");
}

#[test]
fn command_monitors_run_environment_long() {
    let _server =
        integration::mock_endpoint(EndpointOptions::new("POST", "/api/1337/envelope/", 200));
    integration::register_test("monitors/monitors-run-environment-long.trycmd");
}

#[test]
fn command_monitors_run_help() {
    integration::register_test("monitors/monitors-run-help.trycmd");
}
