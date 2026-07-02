use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_organizations() {
    TestManager::new()
        // Mock is for the organizations list command. The listing is served
        // from a single (control silo) endpoint, so there is no per-region
        // fan-out and no `/users/me/regions/` call to mock.
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/?cursor=")
                .with_response_file("organizations/get-organizations.json"),
        )
        .register_trycmd_test("organizations/*.trycmd")
        .with_default_token();
}
