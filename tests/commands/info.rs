use assert_cmd::Command;
use mockito::mock;
use predicates::str::contains;

use crate::common;

const ENDPOINT: &str = "/api/0/";
const VALID_RESPONSE: &str = r#"{"user":{"username":"kamil@sentry.io","id":"1337","name":"Kamil Ogórek","email":"kamil@sentry.io"},"auth":{"scopes":["project:read","project:releases"]}}"#;

#[test]
fn works_when_all_required_env_are_present() {
    let _server = mock("GET", ENDPOINT)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(VALID_RESPONSE)
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .arg("info")
        .assert()
        .success()
        .stdout(contains("Default Organization: wat"))
        .stdout(contains("Default Project: wat"))
        .stdout(contains("Method: Auth Token"))
        .stdout(contains("User: kamil@sentry.io"))
        .stdout(contains("project:read"))
        .stdout(contains("project:releases"));
}

#[test]
fn fails_without_auth_token() {
    let _server = mock("GET", ENDPOINT)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(VALID_RESPONSE)
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .env_remove("SENTRY_AUTH_TOKEN")
        .arg("info")
        .assert()
        .failure();
}
