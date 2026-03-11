use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_code_mappings_upload() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "POST",
                "/api/0/organizations/wat-org/code-mappings/bulk/",
            )
            .with_response_file("code_mappings/post-bulk.json"),
        )
        .register_trycmd_test("code_mappings/code-mappings-upload.trycmd")
        .with_default_token();
}
