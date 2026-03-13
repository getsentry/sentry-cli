use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_issues_events_help() {
    TestManager::new().register_trycmd_test("issues/issues-events-help.trycmd");
}

#[test]
fn display_latest_event() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/issues/4242424243/events/latest/")
                .with_response_file("issues/get-issue-latest-event.json"),
        )
        .register_trycmd_test("issues/issues-events-latest.trycmd")
        .with_default_token();
}

#[test]
fn display_latest_event_json() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/issues/4242424243/events/latest/")
                .with_response_file("issues/get-issue-latest-event.json"),
        )
        .register_trycmd_test("issues/issues-events-latest-json.trycmd")
        .with_default_token();
}

#[test]
fn event_not_found() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/issues/9999999999/events/latest/")
                .with_status(404)
                .with_response_body(""),
        )
        .register_trycmd_test("issues/issues-events-not-found.trycmd")
        .with_default_token();
}
