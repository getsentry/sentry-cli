use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_projects_list() {
    TestManager::new()
        // mock for projects list
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/projects/?cursor=", 200)
                .with_response_file("projects/get-projects.json"),
        )
        .register_trycmd_test("projects/*.trycmd")
        .with_default_token();
}
