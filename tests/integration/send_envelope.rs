use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_send_envelope() {
    TestManager::new()
        .mock_endpoint(MockEndpointBuilder::new("POST", "/api/1337/envelope/"))
        .register_trycmd_test("send_envelope/*.trycmd");
}
