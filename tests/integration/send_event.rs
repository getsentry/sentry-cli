use crate::integration::register_test;

#[test]
fn command_send_event_help() {
    register_test("send_event/send_event-help.trycmd");
}

#[test]
fn command_send_event_raw() {
    register_test("send_event/send_event-raw.trycmd");
}

#[test]
fn command_send_event_file() {
    register_test("send_event/send_event-file.trycmd");
}
