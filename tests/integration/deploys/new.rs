use mockito::Matcher;
use serde_json::json;

use crate::integration::{mock_endpoint, register_test, EndpointOptions};

#[test]
fn command_deploys_new() {
    let _server = mock_endpoint(
        EndpointOptions::new(
            "POST",
            "/api/0/organizations/wat-org/releases/wat-release/deploys/",
            200,
        )
        .with_response_file("deploys/post-deploys.json")
        .with_matcher(Matcher::PartialJson(json!({
            "environment": "production",
            "name": "custom-deploy",
        }))),
    );
    register_test("deploys/deploys-new.trycmd");
}

#[test]
fn command_releases_deploys_new() {
    let _server = mock_endpoint(
        EndpointOptions::new(
            "POST",
            "/api/0/organizations/wat-org/releases/wat-release/deploys/",
            200,
        )
        .with_response_file("deploys/post-deploys.json")
        .with_matcher(Matcher::PartialJson(json!({
            "environment": "production",
            "name": "custom-deploy",
        }))),
    );
    register_test("releases/releases-deploys-new.trycmd");
}
