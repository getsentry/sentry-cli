use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn successfully_deletes() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "DELETE",
                "/api/0/organizations/wat-org/releases/wat-release/",
            )
            .with_status(204),
        )
        .register_trycmd_test("releases/releases-delete.trycmd")
        .with_default_token();
}

#[test]
fn allows_for_release_to_start_with_hyphen() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "DELETE",
                "/api/0/organizations/wat-org/releases/-hyphenated-release/",
            )
            .with_status(204),
        )
        .register_trycmd_test("releases/releases-delete-hyphen.trycmd")
        .with_default_token();
}

#[test]
fn informs_about_nonexisting_releases() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("DELETE", "/api/0/organizations/wat-org/releases/whoops/")
                .with_status(404),
        )
        .register_trycmd_test("releases/releases-delete-nonexisting.trycmd")
        .with_default_token();
}

#[test]
fn doesnt_allow_to_delete_active_releases() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "DELETE",
                "/api/0/organizations/wat-org/releases/wat-release/",
            )
            .with_status(400)
            .with_response_file("releases/delete-active-release.json"),
        )
        .register_trycmd_test("releases/releases-delete-active.trycmd")
        .with_default_token();
}
