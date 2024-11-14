use crate::integration::{MockEndpointBuilder, TestManager};

// I have no idea why this is timing out on Windows.
// I verified it manually, and this command works just fine. — Kamil
// TODO: Fix windows timeout.
#[cfg(not(windows))]
#[test]
fn command_send_event_not_windows() {
    TestManager::new()
        .mock_endpoint(MockEndpointBuilder::new("POST", "/api/1337/envelope/", 200))
        .register_trycmd_test("send_event/not_windows/*.trycmd");
}

#[test]
fn command_send_event() {
    TestManager::new()
        .mock_endpoint(MockEndpointBuilder::new("POST", "/api/1337/envelope/", 200))
        .register_trycmd_test("send_event/*.trycmd");
}
