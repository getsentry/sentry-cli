use trycmd::TestCases;

use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_logs_help() {
    TestCases::new().case("tests/integration/_cases/logs/logs-help.trycmd");
}

#[test]
fn command_logs_list_help() {
    TestCases::new().case("tests/integration/_cases/logs/logs-list-help.trycmd");
}

#[test]
fn command_logs_list_no_defaults() {
    TestCases::new().case("tests/integration/_cases/logs/logs-list-no-defaults.trycmd");
}

#[test]
fn command_logs_list_basic() {
    TestCases::new().case("tests/integration/_cases/logs/logs-list-basic.trycmd");
}

#[test]
fn command_logs_list_with_data() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/organizations/wat-org/events/?dataset=ourlogs&field=sentry.item_id&field=trace&field=severity&field=timestamp&field=message&project=wat-project&per_page=100&statsPeriod=1h&referrer=sentry-cli-tail&sort=-timestamp",
            )
            .with_response_file("logs/get-logs.json"),
        )
        .register_trycmd_test("logs/logs-list-with-data.trycmd")
        .with_default_token();
}
