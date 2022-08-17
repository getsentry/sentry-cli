use crate::integration::{mock_endpoint, register_test, EndpointOptions};

#[test]
fn command_events_list_help() {
    register_test("events/events-list-help.trycmd");
}

#[test]
fn doesnt_fail_with_empty_response() {
    let _server = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/events/?cursor=",
            200,
        )
        .with_response_body("[]"),
    );
    register_test("events/events-list-empty.trycmd");
}
