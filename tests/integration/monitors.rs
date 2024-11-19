use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_monitors() {
    let manager = TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/monitors/?cursor=")
                .with_response_file("monitors/get-monitors.json"),
        )
        .mock_endpoint(MockEndpointBuilder::new("POST", "/api/1337/envelope/"))
        .mock_endpoint(
            MockEndpointBuilder::new("POST", "/api/0/monitors/foo-monitor/checkins/")
                .with_response_file("monitors/post-monitors.json"),
        )
        .register_trycmd_test("monitors/*.trycmd")
        .with_default_token();

    #[cfg(not(windows))]
    manager.register_trycmd_test("monitors/not_windows/*.trycmd");

    #[cfg(windows)]
    manager.register_trycmd_test("monitors/windows/*.trycmd");
}

#[test]
fn command_monitors_run_server_error() {
    let manager = TestManager::new()
        .mock_endpoint(MockEndpointBuilder::new("POST", "/api/1337/envelope/").with_status(500));

    #[cfg(not(windows))]
    manager.register_trycmd_test("monitors/server_error/monitors-run-server-error.trycmd");

    #[cfg(windows)]
    manager.register_trycmd_test("monitors/server_error/monitors-run-server-error-win.trycmd");
}
