use crate::integration::{mock_endpoint, register_test, EndpointOptions};
use mockito::Matcher;
use serde_json::json;

#[test]
fn successfully_creates_a_release() {
    let _server = mock_endpoint(
        EndpointOptions::new(
            "PUT",
            "/api/0/projects/wat-org/wat-project/releases/wat-release/",
            200,
        )
        .with_response_file("releases/get-release.json"),
    );
    register_test("releases/releases-finalize.trycmd");
}

#[test]
fn allows_for_release_to_start_with_hyphen() {
    let _server = mock_endpoint(
        EndpointOptions::new(
            "PUT",
            "/api/0/projects/wat-org/wat-project/releases/-hyphenated-release/",
            200,
        )
        .with_response_file("releases/get-release.json"),
    );
    register_test("releases/releases-finalize-hyphen.trycmd");
}

#[test]
fn release_with_custom_dates() {
    let _server = mock_endpoint(
        EndpointOptions::new(
            "PUT",
            "/api/0/projects/wat-org/wat-project/releases/wat-release/",
            200,
        )
        .with_response_file("releases/get-release.json")
        .with_matcher(Matcher::PartialJson(json!({
            "projects": ["wat-project"],
            "dateStarted": "2015-05-15T00:01:40Z",
            "dateReleased": "2015-05-15T00:00:00Z"
        }))),
    );
    register_test("releases/releases-finalize-dates.trycmd");
}
