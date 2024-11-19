use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_upload_proguard() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("POST", "/api/0/projects/wat-org/wat-project/files/dsyms/")
                .with_response_body("[]"),
        )
        .register_trycmd_test("upload_proguard/*.trycmd")
        .with_default_token();
}

#[test]
fn command_upload_proguard_no_upload_no_auth_token() {
    TestManager::new().register_trycmd_test("upload_proguard/upload_proguard-no-upload.trycmd");
}
