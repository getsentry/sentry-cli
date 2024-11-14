use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_events() {
    TestManager::new()
        // Mock server is used only for the events/events-list-empty.trycmd
        // test. No harm in leaving it here for other tests.
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/events/?cursor=",
                200,
            )
            .with_response_body("[]"),
        )
        .register_trycmd_test("events/*.trycmd")
        .with_default_token();
}
