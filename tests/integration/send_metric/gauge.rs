use crate::integration::register_test;

#[test]
fn command_send_metric_gauge_help() {
    register_test("send_metric/send_metric-gauge-help.trycmd");
}

#[test]
fn command_send_metric_gauge_alphabetic_value() {
    register_test("send_metric/send_metric-gauge-alphabetic-value.trycmd");
}

#[test]
fn command_send_metric_gauge_normalization() {
    register_test("send_metric/send_metric-gauge-normalization.trycmd");
}

#[test]
fn command_send_metric_gauge_required_options() {
    register_test("send_metric/send_metric-gauge-required-options.trycmd");
}

#[test]
fn command_send_metric_gauge_no_options() {
    register_test("send_metric/send_metric-gauge-no-options.trycmd");
}

#[test]
fn command_send_metric_gauge_all_options_long() {
    register_test("send_metric/send_metric-gauge-all-options-long.trycmd");
}

#[test]
fn command_send_metric_gauge_all_options_short() {
    register_test("send_metric/send_metric-gauge-all-options-short.trycmd");
}

#[test]
fn command_send_metric_gauge_integer_value() {
    register_test("send_metric/send_metric-gauge-integer-value.trycmd");
}

#[test]
fn command_send_metric_gauge_float_value() {
    register_test("send_metric/send_metric-gauge-float-value.trycmd");
}

#[test]
fn command_send_metric_gauge_negative_value() {
    register_test("send_metric/send_metric-gauge-negative-value.trycmd");
}
