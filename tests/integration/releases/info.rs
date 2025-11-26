use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn shows_release_details() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/releases/wat-release/")
                .with_response_file("releases/get-release.json"),
        )
        .register_trycmd_test("releases/releases-info.trycmd")
        .with_default_token();
}

#[test]
fn shows_release_details_with_projects_and_commits() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/releases/wat-release/")
                .with_response_file("releases/get-release.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/organizations/wat-org/releases/wat-release/commits/",
            )
            .with_response_file("releases/get-release-commits.json"),
        )
        .register_trycmd_test("releases/releases-info-with-commits-projects.trycmd")
        .with_default_token();
}

#[test]
fn doesnt_print_output_with_quiet_flag() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/releases/wat-release/")
                .with_response_file("releases/get-release.json"),
        )
        .register_trycmd_test("releases/releases-info-quiet.trycmd")
        .with_default_token();
}

#[test]
fn doesnt_print_output_with_silent_flag() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/releases/wat-release/")
                .with_response_file("releases/get-release.json"),
        )
        .register_trycmd_test("releases/releases-info-silent.trycmd")
        .with_default_token();
}

#[test]
fn preserve_valid_exit_code_with_quiet_flag() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/organizations/wat-org/releases/unknown-release/",
            )
            .with_status(404),
        )
        .register_trycmd_test("releases/releases-info-quiet-failed.trycmd")
        .with_default_token();
}

#[test]
fn exits_if_no_release_found() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/releases/wat-release/")
                .with_status(404),
        )
        .register_trycmd_test("releases/releases-info-not-found.trycmd")
        .with_default_token();
}
