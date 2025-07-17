use super::MockEndpointBuilder;
use crate::integration::TestManager;
use mockito::Matcher;
use trycmd::TestCases;

fn envelopes_endpoint_builder() -> MockEndpointBuilder {
    let expected_auth_header = Matcher::Regex(
        r#"^Sentry sentry_key=test, sentry_version=7, sentry_timestamp=\d{10}(\.[0-9]+)?, sentry_client=sentry-cli/.*"#.to_owned(),
    );

    MockEndpointBuilder::new("POST", "/api/1337/envelope/")
        .with_header_matcher("X-Sentry-Auth", expected_auth_header)
}

#[test]
fn command_send_metric() {
    TestManager::new()
        .mock_endpoint(envelopes_endpoint_builder())
        .register_trycmd_test("send_metric/*.trycmd");
}

#[test]
fn command_send_metric_release_and_environment() {
    TestManager::new()
        .mock_endpoint(envelopes_endpoint_builder())
        .register_trycmd_test("send_metric/with_release_and_environment/*.trycmd")
        .env("SENTRY_RELEASE", "def_release")
        .env("SENTRY_ENVIRONMENT", "def_env");
}

#[test]
fn command_send_metric_increment_no_dsn() {
    let _manager = TestManager::new().mock_endpoint(envelopes_endpoint_builder());

    // Custom test case setup because we don't want the DSN to be set by the manager.
    TestCases::new()
        .case("tests/integration/_cases/send_metric/individual_config/send_metric-increment-no-dsn.trycmd");
}

#[test]
fn command_send_metric_increment_unsuccessful_api_call() {
    TestManager::new()
        .mock_endpoint(MockEndpointBuilder::new("POST", "/api/1337/envelope/").with_status(500))
        .register_trycmd_test(
            "send_metric/individual_config/send_metric-increment-unsuccessful-api-call.trycmd",
        );
}
