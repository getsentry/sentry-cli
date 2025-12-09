use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_send_apple_crash() {
    TestManager::new()
        .mock_endpoint(MockEndpointBuilder::new("POST", "/api/1337/envelope/"))
        .register_trycmd_test("send_apple_crash/*.trycmd");
}
