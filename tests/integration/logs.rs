use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_logs_with_api_calls() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET", 
                "/api/0/organizations/wat-org/events/?dataset=ourlogs&field=sentry.item_id&field=trace&field=severity&field=timestamp&field=message&project=wat-project&per_page=100&statsPeriod=90d&sort=-timestamp"
            )
            .with_response_file("logs/get-logs.json"),
        )
        .register_trycmd_test("logs/logs-list-with-data.trycmd")
        .with_default_token();
}

#[test]
fn command_logs_basic() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET", 
                "/api/0/organizations/wat-org/events/?dataset=ourlogs&field=sentry.item_id&field=trace&field=severity&field=timestamp&field=message&project=12345&per_page=1&statsPeriod=90d&sort=-timestamp"
            )
            .with_response_body(r#"{"data": []}"#),
        )
        .register_trycmd_test("logs/logs-list-basic.trycmd")
        .with_default_token();
}

#[test]
fn command_logs_zero_max_rows() {
    TestManager::new().register_trycmd_test("logs/logs-list-with-zero-max-rows.trycmd");
}

#[test]
fn command_logs_help() {
    let manager = TestManager::new();

    #[cfg(not(windows))]
    manager.register_trycmd_test("logs/logs-help.trycmd");
    #[cfg(windows)]
    manager.register_trycmd_test("logs/logs-help-windows.trycmd");
}

#[test]
fn command_logs_list_help() {
    TestManager::new().register_trycmd_test("logs/logs-list-help.trycmd");
}
