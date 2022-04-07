use crate::integration::register_test;

#[test]
fn command_send_event_help() {
    let _t = register_test("send_event/send_event-help.trycmd");
}

#[test]
fn command_send_event_raw() {
    let _t = register_test("send_event/send_event-raw.trycmd");
}

#[test]
fn command_send_event_file() {
    let _t = register_test("send_event/send_event-file.trycmd");
}
