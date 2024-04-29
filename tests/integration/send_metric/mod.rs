use crate::integration::register_test;

mod distribution;
mod gauge;
mod increment;
mod set;

#[test]
fn command_send_metric_help() {
    register_test("send_metric/send_metric-help.trycmd");
}

#[test]
fn command_send_metric_no_subcommand() {
    register_test("send_metric/send_metric-no-subcommand.trycmd");
}

#[test]
fn command_send_metric_global_options() {
    register_test("send_metric/send_metric-global-options.trycmd");
}
