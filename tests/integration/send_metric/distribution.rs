use crate::integration;

#[test]
fn command_send_metric_distribution_all_options_long_with_float_value() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test(
        "send_metric/send_metric-distribution-all-options-long-with-float-value.trycmd",
    );
}

#[test]
fn command_send_metric_distribution_all_options_short_with_int_value() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test(
        "send_metric/send_metric-distribution-all-options-short-with-int-value.trycmd",
    );
}

#[test]
fn command_send_metric_distribution_help() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test("send_metric/send_metric-distribution-help.trycmd");
}

#[test]
fn command_send_metric_distribution_no_options() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test("send_metric/send_metric-distribution-no-options.trycmd");
}

#[test]
fn command_send_metric_distribution_normalization() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test("send_metric/send_metric-distribution-normalization.trycmd");
}

#[test]
fn command_send_metric_distribution_numerical_key_prefix() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test("send_metric/send_metric-distribution-numerical-key-prefix.trycmd");
}

#[test]
fn command_send_metric_distribution_required_options_with_negative_value() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test(
        "send_metric/send_metric-distribution-required-options-with-negative-value.trycmd",
    );
}
