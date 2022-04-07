use crate::integration::register_test;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[test]
fn command_send_event_raw() {
    let t = register_test("send_event/send_event-raw.trycmd");
    t.insert_var("[VERSION]", VERSION).unwrap();
}

#[test]
fn command_send_event_file() {
    let _t = register_test("send_event/send_event-file.trycmd");
}
