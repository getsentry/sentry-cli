use mockito::Matcher;
use serde_json::json;

use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_deploys_new() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "POST",
                "/api/0/organizations/wat-org/releases/wat-release/deploys/",
            )
            .with_response_file("deploys/post-deploys.json")
            .with_matcher(Matcher::PartialJson(json!({
                "environment": "production",
                "name": "custom-deploy",
            }))),
        )
        .register_trycmd_test("deploys/deploys-new.trycmd")
        .with_default_token();
}

#[test]
fn command_releases_deploys_new() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "POST",
                "/api/0/organizations/wat-org/releases/wat-release/deploys/",
            )
            .with_response_file("deploys/post-deploys.json")
            .with_matcher(Matcher::PartialJson(json!({
                "environment": "production",
                "name": "custom-deploy",
            }))),
        )
        .register_trycmd_test("releases/releases-deploys-new.trycmd")
        .with_default_token();
}
