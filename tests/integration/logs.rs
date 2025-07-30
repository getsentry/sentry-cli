use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_logs_with_api_calls() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/organizations/wat-org/events/?dataset=ourlogs&field=sentry.item_id&field=trace&field=severity&field=timestamp&field=message&project=12345&per_page=100&statsPeriod=1h&sort=-timestamp",
            )
            .with_response_file("logs/get-logs.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/organizations/wat-org/events/?dataset=ourlogs&field=sentry.item_id&field=trace&field=severity&field=timestamp&field=message&project=12345&per_page=50&statsPeriod=1h&sort=-timestamp",
            )
            .with_response_file("logs/get-logs.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/organizations/wat-org/events/?dataset=ourlogs&field=sentry.item_id&field=trace&field=severity&field=timestamp&field=message&project=12345&per_page=0&statsPeriod=1h&sort=-timestamp",
            )
            .with_response_body("{\"data\": []}"),
        )
        .register_trycmd_test("logs/*.trycmd")
        .with_default_token();
}
