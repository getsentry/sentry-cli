use crate::integration::register_test;

use super::{mock_endpoint, EndpointOptions};

#[test]
fn command_events() {
    // Mock server is used only for the events/events-list-empty.trycmd
    // test. No harm in leaving it here for other tests.
    let _server = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/events/?cursor=",
            200,
        )
        .with_response_body("[]"),
    );

    register_test("events/*.trycmd");
}
