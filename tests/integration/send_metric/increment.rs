use crate::integration::{self, EndpointOptions};
use trycmd::TestCases;

#[test]
fn command_send_metric_increment_all_options_long_with_float_value() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test(
        "send_metric/send_metric-increment-all-options-long-with-float-value.trycmd",
    );
}

#[test]
fn command_send_metric_increment_all_options_short_with_int_value() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test(
        "send_metric/send_metric-increment-all-options-short-with-int-value.trycmd",
    );
}

#[test]
fn command_send_metric_increment_help() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test("send_metric/send_metric-increment-help.trycmd");
}

#[test]
fn command_send_metric_increment_no_dsn() {
    let _m = super::mock_envelopes_endpoint();
    TestCases::new()
        .case("tests/integration/_cases/send_metric/send_metric-increment-no-dsn.trycmd");
}

#[test]
fn command_send_metric_increment_no_options() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test("send_metric/send_metric-increment-no-options.trycmd");
}

#[test]
fn command_send_metric_increment_normalization_with_negative_value() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test(
        "send_metric/send_metric-increment-normalization-with-negative-value.trycmd",
    );
}

#[test]
fn command_send_metric_increment_numerical_key_prefix() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test("send_metric/send_metric-increment-numerical-key-prefix.trycmd");
}

#[test]
fn command_send_metric_increment_required_options() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test("send_metric/send_metric-increment-required-options.trycmd");
}

#[test]
fn command_send_metric_increment_tag_no_colon() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test("send_metric/send_metric-increment-tag-no-colon.trycmd");
}

#[test]
fn command_send_metric_increment_unsuccessful_api_call() {
    let _m = integration::mock_endpoint(EndpointOptions::new("POST", "/api/1337/envelope/", 500));
    integration::register_test("send_metric/send_metric-increment-unsuccessful-api-call.trycmd");
}
