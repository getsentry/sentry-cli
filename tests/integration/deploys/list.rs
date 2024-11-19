use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_deploys_list() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/organizations/wat-org/releases/wat-release/deploys/",
            )
            .with_response_file("deploys/get-deploys.json"),
        )
        .register_trycmd_test("deploys/deploys-list.trycmd")
        .with_default_token();
}
