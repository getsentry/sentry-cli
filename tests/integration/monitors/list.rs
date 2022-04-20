use crate::integration::{mock_endpoint, register_test, EndpointOptions};

#[test]
fn command_monitors_list() {
    let _server = mock_endpoint(
        EndpointOptions::new("GET", "/api/0/organizations/wat-org/monitors/?cursor=", 200)
            .with_response_file("monitors/get-monitors.json"),
    );
    register_test("monitors/monitors-list.trycmd");
}

#[test]
fn command_monitors_list_help() {
    register_test("monitors/monitors-list-help.trycmd");
}
