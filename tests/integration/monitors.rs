use crate::integration::{mock_endpoint, register_test, MockEndpointBuilder};

#[test]
fn command_monitors() {
    let _list_endpoint = mock_endpoint(
        MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/monitors/?cursor=", 200)
            .with_response_file("monitors/get-monitors.json"),
    );
    let _envelope_endpoint =
        mock_endpoint(MockEndpointBuilder::new("POST", "/api/1337/envelope/", 200));
    let _token_endpoint = mock_endpoint(
        MockEndpointBuilder::new("POST", "/api/0/monitors/foo-monitor/checkins/", 200)
            .with_response_file("monitors/post-monitors.json"),
    );

    let cases = register_test("monitors/*.trycmd");

    #[cfg(not(windows))]
    cases.case("tests/integration/_cases/monitors/not_windows/*.trycmd");

    #[cfg(windows)]
    cases.case("tests/integration/_cases/monitors/windows/*.trycmd");
}

#[test]
fn command_monitors_run_server_error() {
    let _server = mock_endpoint(MockEndpointBuilder::new("POST", "/api/1337/envelope/", 500));

    #[cfg(not(windows))]
    register_test("monitors/server_error/monitors-run-server-error.trycmd");

    #[cfg(windows)]
    register_test("monitors/server_error/monitors-run-server-error-win.trycmd");
}
