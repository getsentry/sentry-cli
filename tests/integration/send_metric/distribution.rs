use crate::integration::register_test;

#[test]
fn command_send_metric_distribution_help() {
    register_test("send_metric/send_metric-distribution-help.trycmd");
}

#[test]
fn command_send_metric_distribution_alphabetic_value() {
    register_test("send_metric/send_metric-distribution-alphabetic-value.trycmd");
}

#[test]
fn command_send_metric_distribution_normalization() {
    register_test("send_metric/send_metric-distribution-normalization.trycmd");
}

#[test]
fn command_send_metric_distribution_required_options() {
    register_test("send_metric/send_metric-distribution-required-options.trycmd");
}

#[test]
fn command_send_metric_distribution_all_options_short() {
    register_test("send_metric/send_metric-distribution-all-options-short.trycmd");
}

#[test]
fn command_send_metric_distribution_no_options() {
    register_test("send_metric/send_metric-distribution-no-options.trycmd");
}

#[test]
fn command_send_metric_distribution_all_options_long() {
    register_test("send_metric/send_metric-distribution-all-options-long.trycmd");
}

#[test]
fn command_send_metric_distribution_integer_value() {
    register_test("send_metric/send_metric-distribution-integer-value.trycmd");
}

#[test]
fn command_send_metric_distribution_float_value() {
    register_test("send_metric/send_metric-distribution-float-value.trycmd");
}

#[test]
fn command_send_metric_distribution_negative_value() {
    register_test("send_metric/send_metric-distribution-negative-value.trycmd");
}
