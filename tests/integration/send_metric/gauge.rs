use crate::integration;

#[test]
fn command_send_metric_gauge_all_options_long_with_float_value() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test(
        "send_metric/send_metric-gauge-all-options-long-with-float-value.trycmd",
    );
}

#[test]
fn command_send_metric_gauge_all_options_short_with_int_value() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test(
        "send_metric/send_metric-gauge-all-options-short-with-int-value.trycmd",
    );
}

#[test]
fn command_send_metric_gauge_default_tags() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test("send_metric/send_metric-gauge-default-tags.trycmd")
        .env("SENTRY_RELEASE", "def_release")
        .env("SENTRY_ENVIRONMENT", "def_env");
}

#[test]
fn command_send_metric_gauge_help() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test("send_metric/send_metric-gauge-help.trycmd");
}

#[test]
fn command_send_metric_gauge_no_options() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test("send_metric/send_metric-gauge-no-options.trycmd");
}

#[test]
fn command_send_metric_gauge_normalization() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test("send_metric/send_metric-gauge-normalization.trycmd");
}

#[test]
fn command_send_metric_gauge_numerical_key_prefix() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test("send_metric/send_metric-gauge-numerical-key-prefix.trycmd");
}

#[test]
fn command_send_metric_gauge_required_options_with_negative_value() {
    let _m = super::mock_envelopes_endpoint();
    integration::register_test(
        "send_metric/send_metric-gauge-required-options-with-negative-value.trycmd",
    );
}
