use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_logs_with_api_calls() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET", 
                "/api/0/organizations/wat-org/events/?dataset=logs&field=sentry.item_id&field=trace&field=severity&field=timestamp&field=message&project=wat-project&query=&per_page=100&statsPeriod=90d&sort=-timestamp"
            )
            .with_response_file("logs/get-logs.json"),
        )
        .register_trycmd_test("logs/logs-list-with-data.trycmd")
        .with_default_token();
}

#[test]
fn command_logs_no_logs_found() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET", 
                "/api/0/organizations/wat-org/events/?dataset=logs&field=sentry.item_id&field=trace&field=severity&field=timestamp&field=message&project=12345&query=&per_page=100&statsPeriod=90d&sort=-timestamp"
            )
            .with_response_body(r#"{"data": []}"#),
        )
        .register_trycmd_test("logs/logs-list-no-logs-found.trycmd")
        .with_default_token();
}

#[test]
fn command_logs_zero_max_rows() {
    TestManager::new().register_trycmd_test("logs/logs-list-with-zero-max-rows.trycmd");
}

#[test]
fn command_logs_list_help() {
    TestManager::new().register_trycmd_test("logs/logs-list-help.trycmd");
}

#[test]
fn command_logs_help() {
    TestManager::new().register_trycmd_test("logs/logs-help.trycmd");
}
