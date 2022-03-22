use mockito::Matcher;
use serde_json::json;

use crate::common::{create_testcase, mock_endpoint, EndpointOptions, UTC_DATE_FORMAT};

#[test]
fn creates_release() {
    let _server = mock_endpoint(
        EndpointOptions::new("POST", "/api/0/projects/wat-org/wat-project/releases/", 201)
            .with_response_file("tests/responses/releases/get-release.json")
            .with_matcher(Matcher::PartialJson(json!({
                "version":"new-release",
                "projects": ["wat-project"],
            }))),
    );
    let t = create_testcase();
    t.case("tests/cmd/releases/releases-new.trycmd");
}

#[test]
fn allows_for_release_to_start_with_hyphen() {
    let _server = mock_endpoint(
        EndpointOptions::new("POST", "/api/0/projects/wat-org/wat-project/releases/", 201)
            .with_response_file("tests/responses/releases/get-release.json")
            .with_matcher(Matcher::PartialJson(json!({
                "version":"-wat-release",
                "projects": ["wat-project"],
            }))),
    );
    let t = create_testcase();
    t.case("tests/cmd/releases/releases-new-hyphen.trycmd");
}

#[test]
fn creates_release_even_if_one_already_exists() {
    let _server = mock_endpoint(
        EndpointOptions::new("POST", "/api/0/projects/wat-org/wat-project/releases/", 208)
            .with_response_file("tests/responses/releases/get-release.json")
            .with_matcher(Matcher::PartialJson(json!({
                "version":"wat-release",
                "projects": ["wat-project"],
            }))),
    );
    let t = create_testcase();
    t.case("tests/cmd/releases/releases-new-existing.trycmd");
}

#[test]
fn creates_release_with_custom_url() {
    let _server = mock_endpoint(
        EndpointOptions::new("POST", "/api/0/projects/wat-org/wat-project/releases/", 208)
            .with_response_file("tests/responses/releases/get-release.json")
            .with_matcher(Matcher::PartialJson(json!({
                "version":"wat-release",
                "projects": ["wat-project"],
                "url":"https://oh.rly"
            }))),
    );
    let t = create_testcase();
    t.case("tests/cmd/releases/releases-new-url.trycmd");
}

#[test]
fn creates_release_which_is_instantly_finalized() {
    let _server = mock_endpoint(
        EndpointOptions::new("POST", "/api/0/projects/wat-org/wat-project/releases/", 208)
            .with_response_file("tests/responses/releases/get-release.json")
            .with_matcher(Matcher::AllOf(vec![
                Matcher::PartialJson(json!({
                    "version":"wat-release",
                    "projects": ["wat-project"],
                })),
                Matcher::Regex(format!(r#""dateReleased":"{}""#, UTC_DATE_FORMAT)),
            ])),
    );
    let t = create_testcase();
    t.case("tests/cmd/releases/releases-new-finalize.trycmd");
}
