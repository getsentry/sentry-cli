use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_environments_list_help() {
    TestManager::new().register_trycmd_test("environments/environments-list-help.trycmd");
}

#[test]
fn display_environments() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/projects/wat-org/wat-project/environments/")
                .with_response_file("environments/get-environments.json"),
        )
        .register_trycmd_test("environments/environments-list.trycmd")
        .with_default_token();
}

#[test]
fn display_hidden_environments() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/projects/wat-org/wat-project/environments/")
                .with_response_file("environments/get-environments.json"),
        )
        .register_trycmd_test("environments/environments-list-show-hidden.trycmd")
        .with_default_token();
}

#[test]
fn project_not_found() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/projects/wat-org/wat-project/environments/")
                .with_status(404),
        )
        .register_trycmd_test("environments/environments-list-not-found.trycmd")
        .with_default_token();
}

#[test]
fn hidden_environments_hint() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/projects/wat-org/wat-project/environments/")
                .with_response_file("environments/get-environments-all-hidden.json"),
        )
        .register_trycmd_test("environments/environments-list-hidden-hint.trycmd")
        .with_default_token();
}

#[test]
fn empty_environments() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/projects/wat-org/wat-project/environments/")
                .with_response_body("[]"),
        )
        .register_trycmd_test("environments/environments-list-empty.trycmd")
        .with_default_token();
}
