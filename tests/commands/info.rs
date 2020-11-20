use assert_cmd::Command;
use mockito::mock;
use predicates::prelude::*;
use predicates::str::contains;

use crate::common;

#[test]
fn info_works_when_all_required_env_are_present() {
    let _server = mock("GET", "/api/0/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"user":{"username":"kamil@sentry.io","id":"1337","name":"Kamil Ogórek","email":"kamil@sentry.io"},"auth":{"scopes":["project:read","project:releases"]}}"#)
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .arg("info")
        .assert()
        .success()
        .stdout(
            contains("Default Organization: wat")
                .and(contains("Default Project: wat"))
                .and(contains("Method: Auth Token"))
                .and(contains("User: kamil@sentry.io"))
                .and(contains("project:read"))
                .and(contains("project:releases")),
        );
}

#[test]
fn info_fails_without_auth_token() {
    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .env_remove("SENTRY_AUTH_TOKEN")
        .arg("info")
        .assert()
        .failure();
}

#[test]
fn info_sets_upload_url_with_args() {
    let _server = mock("GET", "/api/0/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"user":{"username":"kamil@sentry.io","id":"1337","name":"Kamil Ogórek","email":"kamil@sentry.io"},"auth":{"scopes":["project:read","project:releases"]}}"#)
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .arg("--upload-url")
        .arg("https://sentry.io/test")
        .arg("info")
        .assert()
        .success()
        .stdout(
            contains("Sentry upload URL (chunks): https://sentry.io/test")
        );
}
