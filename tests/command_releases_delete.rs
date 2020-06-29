use mockito::{mock, server_url};

use assert_cmd::Command;
use predicates::str::contains;
use std::collections::HashMap;

const ENDPOINT: &str = "/api/0/projects/wat-org/wat-project/releases/wat-release/";

fn get_base_env() -> HashMap<String, String> {
    let mut env = HashMap::new();
    env.insert(String::from("SENTRY_URL"), server_url());
    env.insert(String::from("SENTRY_AUTH_TOKEN"), String::from("lolnope"));
    env.insert(String::from("SENTRY_ORG"), String::from("wat-org"));
    env.insert(String::from("SENTRY_PROJECT"), String::from("wat-project"));
    env
}

#[test]
fn delete_successfully_deletes() {
    let _server = mock("DELETE", ENDPOINT)
        .with_status(204)
        .with_header("content-type", "application/json")
        .with_body("")
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(get_base_env())
        .arg("releases")
        .arg("delete")
        .arg("wat-release")
        .assert()
        .success()
        .stdout(contains("Deleted release wat-release!"));
}

#[test]
fn delete_allows_for_release_to_start_with_hyphen() {
    let _server = mock(
        "DELETE",
        "/api/0/projects/wat-org/wat-project/releases/-wat-release/",
    )
    .with_status(204)
    .with_header("content-type", "application/json")
    .with_body("")
    .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(get_base_env())
        .arg("releases")
        .arg("delete")
        .arg("-wat-release")
        .assert()
        .success()
        .stdout(contains("Deleted release -wat-release!"));
}

#[test]
fn delete_informs_about_nonexisting_releases() {
    let _server = mock("DELETE", ENDPOINT)
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body("")
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(get_base_env())
        .arg("releases")
        .arg("delete")
        .arg("wat-release")
        .assert()
        .success()
        .stdout(contains(
            "Did nothing. Release with this version (wat-release) does not exist.",
        ));
}

#[test]
fn delete_doesnt_allow_to_delete_active_releases() {
    let _server = mock("DELETE", ENDPOINT)
        .with_status(400)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"detail":"This release is referenced by active issues and cannot be removed."}"#,
        )
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(get_base_env())
        .arg("releases")
        .arg("delete")
        .arg("wat-release")
        .assert()
        .failure()
        .stderr(contains(
            "sentry reported an error: This release is referenced by active issues and cannot be removed. (http status: 400)",
        ));
}
