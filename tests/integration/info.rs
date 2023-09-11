use mockito::server_url;
use trycmd::TestCases;

use crate::integration::{mock_endpoint, register_test, EndpointOptions};

#[test]
fn command_info_help() {
    register_test("info/info-help.trycmd");
}

#[test]
fn command_info_no_token() {
    // Special case where we don't want any env variables set, so we don't use `register_task` helper.
    TestCases::new()
        .env("SENTRY_INTEGRATION_TEST", "1")
        .case("tests/integration/_cases/info/info-no-token.trycmd");
}

#[test]
fn command_info_basic() {
    let _server = mock_endpoint(
        EndpointOptions::new("GET", "/api/0/", 200).with_response_file("info/get-info.json"),
    );
    let t = register_test("info/info-basic.trycmd");
    t.insert_var("[SERVER]", server_url()).unwrap();
}

#[test]
fn command_info_no_defaults() {
    let _server = mock_endpoint(
        EndpointOptions::new("GET", "/api/0/", 200).with_response_file("info/get-info.json"),
    );
    let t = register_test("info/info-json.trycmd");
    t.insert_var("[SERVER]", server_url()).unwrap();
}

#[test]
fn command_info_json() {
    let _server = mock_endpoint(
        EndpointOptions::new("GET", "/api/0/", 200).with_response_file("info/get-info.json"),
    );
    let t = register_test("info/info-basic.trycmd");
    t.insert_var("[SERVER]", server_url()).unwrap();
}

#[test]
fn command_info_json_without_defaults() {
    let _server = mock_endpoint(
        EndpointOptions::new("GET", "/api/0/", 200).with_response_file("info/get-info.json"),
    );
    let t = register_test("info/info-json-no-defaults.trycmd");
    t.env("SENTRY_ORG", "").env("SENTRY_PROJECT", "");
    t.insert_var("[SERVER]", server_url()).unwrap();
}
