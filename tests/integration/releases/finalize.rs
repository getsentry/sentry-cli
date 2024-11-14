use crate::integration::{MockEndpointBuilder, TestManager};
use mockito::Matcher;
use serde_json::json;

#[test]
fn successfully_creates_a_release() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "PUT",
                "/api/0/projects/wat-org/wat-project/releases/wat-release/",
                200,
            )
            .with_response_file("releases/get-release.json"),
        )
        .register_trycmd_test("releases/releases-finalize.trycmd")
        .with_default_token();
}

#[test]
fn allows_for_release_to_start_with_hyphen() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "PUT",
                "/api/0/projects/wat-org/wat-project/releases/-hyphenated-release/",
                200,
            )
            .with_response_file("releases/get-release.json"),
        )
        .register_trycmd_test("releases/releases-finalize-hyphen.trycmd")
        .with_default_token();
}

#[test]
fn release_with_custom_dates() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
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
        )
        .register_trycmd_test("releases/releases-finalize-dates.trycmd")
        .with_default_token();
}
