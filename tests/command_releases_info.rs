use mockito::mock;

use assert_cmd::Command;
use predicates::str::{contains, is_empty};

mod common;

const ENDPOINT: &str = "/api/0/projects/wat-org/wat-project/releases/wat-release/";
const VALID_RESPONSE: &str = r#"{"dateReleased":"2020-06-29T12:16:49.368667Z","newGroups":0,"commitCount":0,"url":null,"data":{},"lastDeploy":null,"deployCount":0,"dateCreated":"2020-06-29T11:36:59.612687Z","lastEvent":null,"version":"wat-release","firstEvent":null,"lastCommit":null,"shortVersion":"wat-release","authors":[],"owner":null,"versionInfo":{"buildHash":null,"version":{"raw":"wat-release"},"description":"wat-release","package":null},"ref":null,"projects":[{"name":"test","platform":"javascript","slug":"test","platforms":["javascript"],"newGroups":0,"id":1861017}]}"#;

#[test]
fn releases_info_shows_release_details() {
    let _server = mock("GET", ENDPOINT)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(VALID_RESPONSE)
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .arg("releases")
        .arg("info")
        .arg("wat-release")
        .assert()
        .success()
        .stdout(contains(
            "| Version      | wat-release                    |",
        ))
        .stdout(contains(
            "| Date created | 2020-06-29 11:36:59.612687 UTC |",
        ));
}

#[test]
fn releases_info_doesnt_print_output_with_quiet_flag() {
    let _server = mock("GET", ENDPOINT)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(VALID_RESPONSE)
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .arg("releases")
        .arg("info")
        .arg("wat-release")
        .arg("--quiet")
        .assert()
        .success()
        .stdout(is_empty());
}

#[test]
fn releases_info_exits_if_no_release_found() {
    let _server = mock("GET", ENDPOINT)
        .with_status(404)
        .with_header("content-type", "application/json")
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .arg("releases")
        .arg("info")
        .arg("wat-release")
        .assert()
        .failure();
}
