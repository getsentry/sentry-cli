use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_issues_show_help() {
    TestManager::new().register_trycmd_test("issues/issues-show-help.trycmd");
}

#[test]
fn display_issue() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/issues/4242424243/")
                .with_response_file("issues/get-issue.json"),
        )
        .register_trycmd_test("issues/issues-show.trycmd")
        .with_default_token();
}

#[test]
fn show_with_bulk_flags_errors() {
    TestManager::new().register_trycmd_test("issues/issues-show-invalid-flags.trycmd");
}

#[test]
fn issue_not_found() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/issues/9999999999/")
                .with_status(404)
                .with_response_body("{}"),
        )
        .register_trycmd_test("issues/issues-show-not-found.trycmd")
        .with_default_token();
}

#[test]
fn doesnt_print_output_with_quiet_flag() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/issues/4242424243/")
                .with_response_file("issues/get-issue.json"),
        )
        .register_trycmd_test("issues/issues-show-quiet.trycmd")
        .with_default_token();
}

#[test]
fn preserve_valid_exit_code_with_quiet_flag() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/issues/9999999999/")
                .with_status(404)
                .with_response_body("{}"),
        )
        .register_trycmd_test("issues/issues-show-quiet-not-found.trycmd")
        .with_default_token();
}
