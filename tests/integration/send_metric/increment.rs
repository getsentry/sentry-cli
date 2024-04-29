use trycmd::TestCases;

use crate::integration::register_test;

#[test]
fn command_send_metric_increment_help() {
    register_test("send_metric/send_metric-increment-help.trycmd");
}

#[test]
fn command_send_metric_increment_alphabetic_value() {
    register_test("send_metric/send_metric-increment-alphabetic-value.trycmd");
}

#[test]
fn command_send_metric_increment_tag_no_colon() {
    register_test("send_metric/send_metric-increment-tag-no-colon.trycmd");
}

#[test]
fn command_send_metric_increment_normalization() {
    register_test("send_metric/send_metric-increment-normalization.trycmd");
}

#[test]
fn command_send_metric_increment_required_options() {
    register_test("send_metric/send_metric-increment-required-options.trycmd");
}

#[test]
fn command_send_metric_increment_default_tags_override() {
    register_test("send_metric/send_metric-increment-default-tags-override.trycmd");
}

#[test]
fn command_send_metric_increment_all_options_long() {
    register_test("send_metric/send_metric-increment-all-options-long.trycmd");
}

#[test]
fn command_send_metric_increment_all_options_short() {
    register_test("send_metric/send_metric-increment-all-options-short.trycmd");
}

#[test]
fn command_send_metric_increment_no_options() {
    register_test("send_metric/send_metric-increment-no-options.trycmd");
}

#[test]
fn command_send_metric_increment_no_dsn() {
    TestCases::new()
        .case("tests/integration/_cases/send_metric/send_metric-increment-no-dsn.trycmd");
}

#[test]
fn command_send_metric_increment_integer_value() {
    register_test("send_metric/send_metric-increment-integer-value.trycmd");
}

#[test]
fn command_send_metric_increment_float_value() {
    register_test("send_metric/send_metric-increment-float-value.trycmd");
}

#[test]
fn command_send_metric_increment_negative_value() {
    register_test("send_metric/send_metric-increment-negative-value.trycmd");
}
