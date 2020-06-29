use mockito::mock;

use assert_cmd::Command;
use predicates::str::contains;

mod common;

const ENDPOINT: &str = "/api/0/projects/wat-org/wat-project/releases/wat-release/";

#[test]
fn releases_delete_successfully_deletes() {
    let _server = mock("DELETE", ENDPOINT)
        .with_status(204)
        .with_header("content-type", "application/json")
        .with_body("")
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .arg("releases")
        .arg("delete")
        .arg("wat-release")
        .assert()
        .success()
        .stdout(contains("Deleted release wat-release!"));
}

#[test]
fn releases_delete_allows_for_release_to_start_with_hyphen() {
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
        .envs(common::get_base_env())
        .arg("releases")
        .arg("delete")
        .arg("-wat-release")
        .assert()
        .success()
        .stdout(contains("Deleted release -wat-release!"));
}

#[test]
fn releases_delete_informs_about_nonexisting_releases() {
    let _server = mock("DELETE", ENDPOINT)
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body("")
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
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
fn releases_delete_doesnt_allow_to_delete_active_releases() {
    let _server = mock("DELETE", ENDPOINT)
        .with_status(400)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"detail":"This release is referenced by active issues and cannot be removed."}"#,
        )
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .arg("releases")
        .arg("delete")
        .arg("wat-release")
        .assert()
        .failure()
        .stderr(contains(
            "sentry reported an error: This release is referenced by active issues and cannot be removed. (http status: 400)",
        ));
}
