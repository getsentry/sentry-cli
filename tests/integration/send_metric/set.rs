use crate::integration::register_test;

#[test]
fn command_send_metric_set_help() {
    register_test("send_metric/send_metric-set-help.trycmd");
}

#[test]
fn command_send_metric_set_alphabetic_value() {
    register_test("send_metric/send_metric-set-alphabetic-value.trycmd");
}

#[test]
fn command_send_metric_set_normalization() {
    register_test("send_metric/send_metric-set-normalization.trycmd");
}

#[test]
fn command_send_metric_set_no_options() {
    register_test("send_metric/send_metric-set-no-options.trycmd");
}

#[test]
fn command_send_metric_set_required_options() {
    register_test("send_metric/send_metric-set-required-options.trycmd");
}

#[test]
fn command_send_metric_set_all_options_long() {
    register_test("send_metric/send_metric-set-all-options-long.trycmd");
}

#[test]
fn command_send_metric_set_all_options_short() {
    register_test("send_metric/send_metric-set-all-options-short.trycmd");
}

#[test]
fn command_send_metric_set_integer_value() {
    register_test("send_metric/send_metric-set-integer-value.trycmd");
}

#[test]
fn command_send_metric_set_float_value() {
    register_test("send_metric/send_metric-set-float-value.trycmd");
}

#[test]
fn command_send_metric_set_negative_value() {
    register_test("send_metric/send_metric-set-negative-value.trycmd");
}
