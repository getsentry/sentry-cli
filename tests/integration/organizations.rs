use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_organizations() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/?cursor=")
                .with_response_file("organizations/get-organizations.json"),
        )
        .register_trycmd_test("organizations/*.trycmd")
        .with_default_token();
}
