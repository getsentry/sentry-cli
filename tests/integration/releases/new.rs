use mockito::Matcher;
use serde_json::json;

use crate::integration::{MockEndpointBuilder, TestManager, UTC_DATE_FORMAT};

#[test]
fn command_releases_new_help() {
    TestManager::new().register_trycmd_test("releases/releases-new-help.trycmd");
}

#[test]
fn creates_release() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("POST", "/api/0/organizations/wat-org/releases/")
                .with_status(201)
                .with_response_file("releases/get-release.json")
                .with_matcher(Matcher::PartialJson(json!({
                    "version": "new-release",
                    "projects": ["wat-project"],
                }))),
        )
        .register_trycmd_test("releases/releases-new.trycmd")
        .with_default_token();
}

#[test]
fn allows_for_release_to_start_with_hyphen() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("POST", "/api/0/organizations/wat-org/releases/")
                .with_status(201)
                .with_response_file("releases/get-release.json")
                .with_matcher(Matcher::PartialJson(json!({
                    "version": "-hyphenated-release",
                    "projects": ["wat-project"],
                }))),
        )
        .register_trycmd_test("releases/releases-new-hyphen.trycmd")
        .with_default_token();
}

#[test]
fn creates_release_with_project() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("POST", "/api/0/organizations/wat-org/releases/")
                .with_status(201)
                .with_response_file("releases/get-release.json")
                .with_matcher(Matcher::PartialJson(json!({
                    "version": "new-release",
                    "projects": ["wat-project"],
                }))),
        )
        .register_trycmd_test("releases/releases-new-with-project.trycmd")
        .with_default_token();
}

#[test]
fn allows_for_release_with_project_to_start_with_hyphen() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("POST", "/api/0/organizations/wat-org/releases/")
                .with_status(201)
                .with_response_file("releases/get-release.json")
                .with_matcher(Matcher::PartialJson(json!({
                    "version": "-hyphenated-release",
                    "projects": ["wat-project"],
                }))),
        )
        .register_trycmd_test("releases/releases-new-with-project-hyphen.trycmd")
        .with_default_token();
}

#[test]
fn creates_release_even_if_one_already_exists() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("POST", "/api/0/organizations/wat-org/releases/")
                .with_status(208)
                .with_response_file("releases/get-release.json")
                .with_matcher(Matcher::PartialJson(json!({
                    "version": "wat-release",
                    "projects": ["wat-project"],
                }))),
        )
        .register_trycmd_test("releases/releases-new-existing.trycmd")
        .with_default_token();
}

#[test]
fn creates_release_with_custom_url() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("POST", "/api/0/organizations/wat-org/releases/")
                .with_status(208)
                .with_response_file("releases/get-release.json")
                .with_matcher(Matcher::PartialJson(json!({
                    "version": "wat-release",
                    "projects": ["wat-project"],
                    "url": "https://oh.rly"
                }))),
        )
        .register_trycmd_test("releases/releases-new-url.trycmd")
        .with_default_token();
}

#[test]
fn creates_release_which_is_instantly_finalized() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("POST", "/api/0/organizations/wat-org/releases/")
                .with_status(208)
                .with_response_file("releases/get-release.json")
                .with_matcher(Matcher::AllOf(vec![
                    Matcher::PartialJson(json!({
                        "version": "wat-release",
                        "projects": ["wat-project"],
                    })),
                    Matcher::Regex(format!(r#""dateReleased":"{UTC_DATE_FORMAT}""#)),
                ])),
        )
        .register_trycmd_test("releases/releases-new-finalize.trycmd")
        .with_default_token();
}
