use trycmd::TestCases;

use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_info_help() {
    TestManager::new().register_trycmd_test("info/info-help.trycmd");
}

#[test]
fn command_info_no_token() {
    // Special case where we don't want any env variables set, so we don't use `TestManager`.
    TestCases::new()
        .env("SENTRY_INTEGRATION_TEST", "1")
        .case("tests/integration/_cases/info/info-no-token.trycmd");
}

#[test]
fn command_info_no_token_backtrace() {
    // Special case where we don't want any env variables set, so we don't use `TestManager`.
    TestCases::new()
        .env("SENTRY_INTEGRATION_TEST", "1")
        .env("RUST_BACKTRACE", "1")
        .case("tests/integration/_cases/info/info-no-token-backtrace.trycmd");
}

#[test]
fn command_info_basic() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/", 200)
                .with_response_file("info/get-info.json"),
        )
        .register_trycmd_test("info/info-basic.trycmd")
        .with_default_token()
        .with_server_var()
        .expect("Failed to set server variable");
}

#[test]
fn command_info_no_defaults() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/", 200)
                .with_response_file("info/get-info.json"),
        )
        .register_trycmd_test("info/info-json.trycmd")
        .with_default_token()
        .with_server_var()
        .expect("Failed to set server variable");
}

#[test]
fn command_info_json() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/", 200)
                .with_response_file("info/get-info.json"),
        )
        .register_trycmd_test("info/info-basic.trycmd")
        .with_default_token()
        .with_server_var()
        .expect("Failed to set server variable");
}

#[test]
fn command_info_json_without_defaults() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/api/0/", 200)
                .with_response_file("info/get-info.json"),
        )
        .register_trycmd_test("info/info-json-no-defaults.trycmd")
        .env("SENTRY_ORG", "")
        .env("SENTRY_PROJECT", "")
        .with_default_token()
        .with_server_var()
        .expect("Failed to set server variable");
}
