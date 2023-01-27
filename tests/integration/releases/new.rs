use mockito::Matcher;
use serde_json::json;

use crate::integration::{mock_endpoint, register_test, EndpointOptions, UTC_DATE_FORMAT};

#[test]
fn command_releases_new_help() {
    register_test("releases/releases-new-help.trycmd");
}

#[test]
fn creates_release() {
    let _server = mock_endpoint(
        EndpointOptions::new("POST", "/api/0/projects/wat-org/wat-project/releases/", 201)
            .with_response_file("releases/get-release.json")
            .with_matcher(Matcher::PartialJson(json!({
                "version": "new-release",
                "projects": ["wat-project"],
            }))),
    );
    register_test("releases/releases-new.trycmd");
}

#[test]
fn allows_for_release_to_start_with_hyphen() {
    let _server = mock_endpoint(
        EndpointOptions::new("POST", "/api/0/projects/wat-org/wat-project/releases/", 201)
            .with_response_file("releases/get-release.json")
            .with_matcher(Matcher::PartialJson(json!({
                "version": "-hyphenated-release",
                "projects": ["wat-project"],
            }))),
    );
    register_test("releases/releases-new-hyphen.trycmd");
}

#[test]
fn creates_release_with_project() {
    let _server = mock_endpoint(
        EndpointOptions::new("POST", "/api/0/projects/wat-org/wat-project/releases/", 201)
            .with_response_file("releases/get-release.json")
            .with_matcher(Matcher::PartialJson(json!({
                "version": "new-release",
                "projects": ["wat-project"],
            }))),
    );
    register_test("releases/releases-new-with-project.trycmd");
}

#[test]
fn allows_for_release_with_project_to_start_with_hyphen() {
    let _server = mock_endpoint(
        EndpointOptions::new("POST", "/api/0/projects/wat-org/wat-project/releases/", 201)
            .with_response_file("releases/get-release.json")
            .with_matcher(Matcher::PartialJson(json!({
                "version": "-hyphenated-release",
                "projects": ["wat-project"],
            }))),
    );
    register_test("releases/releases-new-with-project-hyphen.trycmd");
}

#[test]
fn creates_release_even_if_one_already_exists() {
    let _server = mock_endpoint(
        EndpointOptions::new("POST", "/api/0/projects/wat-org/wat-project/releases/", 208)
            .with_response_file("releases/get-release.json")
            .with_matcher(Matcher::PartialJson(json!({
                "version": "wat-release",
                "projects": ["wat-project"],
            }))),
    );
    register_test("releases/releases-new-existing.trycmd");
}

#[test]
fn creates_release_with_custom_url() {
    let _server = mock_endpoint(
        EndpointOptions::new("POST", "/api/0/projects/wat-org/wat-project/releases/", 208)
            .with_response_file("releases/get-release.json")
            .with_matcher(Matcher::PartialJson(json!({
                "version": "wat-release",
                "projects": ["wat-project"],
                "url": "https://oh.rly"
            }))),
    );
    register_test("releases/releases-new-url.trycmd");
}

#[test]
fn creates_release_which_is_instantly_finalized() {
    let _server = mock_endpoint(
        EndpointOptions::new("POST", "/api/0/projects/wat-org/wat-project/releases/", 208)
            .with_response_file("releases/get-release.json")
            .with_matcher(Matcher::AllOf(vec![
                Matcher::PartialJson(json!({
                    "version": "wat-release",
                    "projects": ["wat-project"],
                })),
                Matcher::Regex(format!(r#""dateReleased":"{UTC_DATE_FORMAT}""#)),
            ])),
    );
    register_test("releases/releases-new-finalize.trycmd");
}
