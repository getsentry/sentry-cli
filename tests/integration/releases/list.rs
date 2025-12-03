use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn displays_releases() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/releases/")
                .with_response_file("releases/get-releases.json"),
        )
        .register_trycmd_test("releases/releases-list.trycmd")
        .with_default_token();
}

#[test]
fn displays_releases_with_projects() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/releases/")
                .with_response_file("releases/get-releases.json"),
        )
        .register_trycmd_test("releases/releases-list-with-projects.trycmd")
        .with_default_token();
}

#[test]
fn doesnt_fail_with_empty_response() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/releases/")
                .with_response_body("[]"),
        )
        .register_trycmd_test("releases/releases-list-empty.trycmd")
        .with_default_token();
}

#[test]
fn can_override_org() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/whynot/releases/")
                .with_response_file("releases/get-releases.json"),
        )
        .register_trycmd_test("releases/releases-list-override-org.trycmd")
        .with_default_token();
}

#[test]
fn displays_releases_in_raw_mode() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/releases/")
                .with_response_file("releases/get-releases.json"),
        )
        .register_trycmd_test("releases/releases-list-raw.trycmd")
        .with_default_token();
}

#[test]
fn displays_releases_in_raw_mode_with_delimiter() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/releases/")
                .with_response_file("releases/get-releases.json"),
        )
        .register_trycmd_test("releases/releases-list-raw-delimiter.trycmd")
        .with_default_token();
}
