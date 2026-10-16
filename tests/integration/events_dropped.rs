use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_events_dropped_project_slug() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/organizations/wat-org/events-dropped/?projectSlug=wat-project&dataset=errors&statsPeriod=90d",
            )
            .with_response_file("events/get-dropped.json"),
        )
        .register_trycmd_test("events_dropped/events-dropped.trycmd")
        .with_default_token();
}

#[test]
fn command_events_dropped_project_id() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/organizations/wat-org/events-dropped/?project=12345&dataset=errors&statsPeriod=90d",
            )
            .with_response_file("events/get-dropped.json"),
        )
        .register_trycmd_test("events_dropped/events-dropped-project-id.trycmd")
        .with_default_token();
}

#[test]
fn command_events_dropped_json() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/organizations/wat-org/events-dropped/?projectSlug=wat-project&dataset=errors&statsPeriod=90d",
            )
            .with_response_file("events/get-dropped.json"),
        )
        .register_trycmd_test("events_dropped/events-dropped-json.trycmd")
        .with_default_token();
}

#[test]
fn command_events_dropped_empty() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/organizations/wat-org/events-dropped/?projectSlug=wat-project&dataset=errors&statsPeriod=90d",
            )
            .with_response_body(r#"{"droppedEvents": [], "acceptedEvents": []}"#),
        )
        .register_trycmd_test("events_dropped/events-dropped-empty.trycmd")
        .with_default_token();
}
