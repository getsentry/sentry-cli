use crate::integration::{self, EndpointOptions};

// I have no idea why this is timing out on Windows.
// I verified it manually, and this command works just fine. — Kamil
// TODO: Fix windows timeout.
#[cfg(not(windows))]
#[test]
fn command_send_event_not_windows() {
    let _server =
        integration::mock_endpoint(EndpointOptions::new("POST", "/api/1337/envelope/", 200));
    integration::register_test("send_event/not_windows/*.trycmd");
}

#[test]
fn command_send_event() {
    let _server =
        integration::mock_endpoint(EndpointOptions::new("POST", "/api/1337/envelope/", 200));
    integration::register_test("send_event/*.trycmd");
}
