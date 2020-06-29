use assert_cmd::Command;
use mockito::mock;
use predicates::str::contains;

use crate::common;

#[test]
fn successfully_deletes() {
    let _server = mock(
        "DELETE",
        "/api/0/projects/wat-org/wat-project/releases/wat-release/",
    )
    .with_status(204)
    .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .args(vec!["releases", "delete", "wat-release"])
        .assert()
        .success()
        .stdout(contains("Deleted release wat-release!"));
}

#[test]
fn allows_for_release_to_start_with_hyphen() {
    let _server = mock(
        "DELETE",
        "/api/0/projects/wat-org/wat-project/releases/-wat-release/",
    )
    .with_status(204)
    .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .args(vec!["releases", "delete", "-wat-release"])
        .assert()
        .success()
        .stdout(contains("Deleted release -wat-release!"));
}

#[test]
fn informs_about_nonexisting_releases() {
    let _server = mock(
        "DELETE",
        "/api/0/projects/wat-org/wat-project/releases/wat-release/",
    )
    .with_status(404)
    .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .args(vec!["releases", "delete", "wat-release"])
        .assert()
        .success()
        .stdout(contains(
            "Did nothing. Release with this version (wat-release) does not exist.",
        ));
}

#[test]
fn doesnt_allow_to_delete_active_releases() {
    let _server = mock(
        "DELETE",
        "/api/0/projects/wat-org/wat-project/releases/wat-release/",
    )
    .with_status(400)
    .with_header("content-type", "application/json")
    .with_body(r#"{"detail":"This release is referenced by active issues and cannot be removed."}"#)
    .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .args(vec!["releases", "delete", "wat-release"])
        .assert()
        .failure()
        .stderr(contains(
            "sentry reported an error: This release is referenced by active issues and cannot be removed. (http status: 400)",
        ));
}
