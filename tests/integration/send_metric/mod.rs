use super::EndpointOptions;
use crate::integration;
use mockito::{Matcher, Mock};

mod distribution;
mod gauge;
mod increment;
mod set;

fn mock_envelopes_endpoint() -> Mock {
    let expected_auth_header = Matcher::Regex(
        r#"^Sentry sentry_key=test, sentry_version=7, sentry_timestamp=\d{10}(\.[0-9]+)?, sentry_client=sentry-cli/.*"#
            .to_string(),
    );
    integration::mock_endpoint(
        EndpointOptions::new("POST", "/api/1337/envelope/", 200)
            .with_header_matcher("X-Sentry-Auth", expected_auth_header),
    )
}

#[test]
fn command_send_metric_help() {
    integration::register_test("send_metric/send_metric-help.trycmd");
}

#[test]
fn command_send_metric_no_subcommand() {
    integration::register_test("send_metric/send_metric-no-subcommand.trycmd");
}

#[test]
fn command_send_metric_global_options() {
    integration::register_test("send_metric/send_metric-global-options.trycmd");
}
