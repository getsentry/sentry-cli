use crate::integration::register_test;

mod list;

#[test]
fn command_events_help() {
    register_test("events/events-help.trycmd");
}

#[test]
fn command_events_no_subcommand() {
    register_test("events/events-no-subcommand.trycmd");
}
