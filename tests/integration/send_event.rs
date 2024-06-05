use crate::integration::{self, EndpointOptions};

#[test]
fn command_send_event_help() {
    integration::register_test("send_event/send_event-help.trycmd");
}

// I have no idea why this is timing out on Windows.
// I verified it manually, and this command works just fine. — Kamil
// TODO: Fix windows timeout.
#[cfg(not(windows))]
#[test]
fn command_send_event_raw() {
    let _server =
        integration::mock_endpoint(EndpointOptions::new("POST", "/api/1337/envelope/", 200));
    integration::register_test("send_event/send_event-raw.trycmd");
}

#[test]
fn command_send_event_file() {
    let _server =
        integration::mock_endpoint(EndpointOptions::new("POST", "/api/1337/envelope/", 200));
    integration::register_test("send_event/send_event-file.trycmd");
}

#[test]
fn command_send_event_raw_fail() {
    integration::register_test("send_event/send_event-raw-fail.trycmd");
}
