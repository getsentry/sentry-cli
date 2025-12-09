use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_send_apple_crash_help() {
    TestManager::new().register_trycmd_test("send_apple_crash/send_apple_crash-help.trycmd");
}

#[test]
fn command_send_apple_crash() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("POST", "/api/1337/envelope/")
                .with_response_file("empty.json"),
        )
        .register_trycmd_test("send_apple_crash/*.trycmd");
}

#[test]
fn command_send_apple_crash_invalid() {
    // Tests for error cases - no mock endpoint needed as they fail before sending
    TestManager::new().register_trycmd_test("send_apple_crash/error/*.trycmd");
}
