use super::MockEndpointBuilder;
use crate::integration;
use mockito::{Matcher, Mock};
use trycmd::TestCases;

fn mock_envelopes_endpoint() -> Mock {
    let expected_auth_header = Matcher::Regex(
        r#"^Sentry sentry_key=test, sentry_version=7, sentry_timestamp=\d{10}(\.[0-9]+)?, sentry_client=sentry-cli/.*"#
            .to_string(),
    );
    integration::mock_endpoint(
        MockEndpointBuilder::new("POST", "/api/1337/envelope/", 200)
            .with_header_matcher("X-Sentry-Auth", expected_auth_header),
    )
}

#[test]
fn command_send_metric() {
    let _m = mock_envelopes_endpoint();
    integration::register_test("send_metric/*.trycmd");
}

#[test]
fn command_send_metric_release_and_environment() {
    let _m = mock_envelopes_endpoint();
    integration::register_test("send_metric/with_release_and_environment/*.trycmd")
        .env("SENTRY_RELEASE", "def_release")
        .env("SENTRY_ENVIRONMENT", "def_env");
}

#[test]
fn command_send_metric_increment_no_dsn() {
    let _m = mock_envelopes_endpoint();
    TestCases::new()
        .case("tests/integration/_cases/send_metric/individual_config/send_metric-increment-no-dsn.trycmd");
}

#[test]
fn command_send_metric_increment_unsuccessful_api_call() {
    let _m =
        integration::mock_endpoint(MockEndpointBuilder::new("POST", "/api/1337/envelope/", 500));
    integration::register_test(
        "send_metric/individual_config/send_metric-increment-unsuccessful-api-call.trycmd",
    );
}
