use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_issues_list_help() {
    TestManager::new().register_trycmd_test("issues/issues-list-help.trycmd");
}

#[test]
fn doesnt_fail_with_empty_response() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/issues/?query=&cursor=",
                200,
            )
            .with_response_body("[]"),
        )
        .register_trycmd_test("issues/issues-list-empty.trycmd")
        .with_default_token();
}

#[test]
fn display_issues() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/issues/?query=&cursor=",
                200,
            )
            .with_response_file("issues/get-issues.json"),
        )
        .register_trycmd_test("issues/issues-display.trycmd")
        .with_default_token();
}

#[test]
fn display_resolved_issues() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/issues/?query=is:resolved&cursor=",
                200,
            )
            .with_response_file("issues/get-resolved-issues.json"),
        )
        .register_trycmd_test("issues/issues-display-with-query.trycmd")
        .with_default_token();
}
