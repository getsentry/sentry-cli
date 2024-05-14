use super::mock_envelopes_endpoint;
use crate::integration::register_test;

#[test]
fn command_send_metric_set_all_options_long_with_alphabetic_value() {
    let _m = mock_envelopes_endpoint();
    register_test("send_metric/send_metric-set-all-options-long-with-alphabetic-value.trycmd");
}

#[test]
fn command_send_metric_set_all_options_short_with_int_value() {
    let _m = mock_envelopes_endpoint();
    register_test("send_metric/send_metric-set-all-options-short-with-int-value.trycmd");
}

#[test]
fn command_send_metric_set_float_value() {
    let _m = mock_envelopes_endpoint();
    register_test("send_metric/send_metric-set-float-value.trycmd");
}

#[test]
fn command_send_metric_set_help() {
    let _m = mock_envelopes_endpoint();
    register_test("send_metric/send_metric-set-help.trycmd");
}

#[test]
fn command_send_metric_set_no_options() {
    let _m = mock_envelopes_endpoint();
    register_test("send_metric/send_metric-set-no-options.trycmd");
}

#[test]
fn command_send_metric_set_normalization() {
    let _m = mock_envelopes_endpoint();
    register_test("send_metric/send_metric-set-normalization.trycmd");
}

#[test]
fn command_send_metric_set_numerical_key_prefix() {
    let _m = mock_envelopes_endpoint();
    register_test("send_metric/send_metric-set-numerical-key-prefix.trycmd");
}

#[test]
fn command_send_metric_set_required_options_with_negative_value() {
    let _m = mock_envelopes_endpoint();
    register_test("send_metric/send_metric-set-required-options-with-negative-value.trycmd");
}
