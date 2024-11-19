use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_organizations() {
    let manager = TestManager::new();
    let region_response = format!(
        r#"{{
            "regions": [{{
                "name": "monolith",
                "url": "{}"
            }}]
        }}"#,
        manager.server_url(),
    );

    manager
        // Mocks are for the organizations list command.
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/?cursor=")
                .with_response_file("organizations/get-organizations.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/users/me/regions/")
                .with_response_body(region_response),
        )
        .register_trycmd_test("organizations/*.trycmd")
        .with_default_token();
}
