use crate::integration::register_test;

mod list;
mod run;

#[test]
fn command_monitors_help() {
    register_test("monitors/monitors-help.trycmd");
}

#[test]
fn command_monitors_no_subcommand() {
    register_test("monitors/monitors-no-subcommand.trycmd");
}
