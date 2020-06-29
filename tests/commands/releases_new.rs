use mockito::{mock, Matcher};

use assert_cmd::Command;
use predicates::str::contains;

use crate::common;

const ENDPOINT: &str = "/api/0/projects/wat-org/wat-project/releases/";
const VALID_RESPONSE: &str = r#"{"dateReleased":null,"newGroups":0,"commitCount":0,"url":null,"data":{},"lastDeploy":null,"deployCount":0,"dateCreated":"2020-06-29T11:36:59.612687Z","lastEvent":null,"version":"wat-release","firstEvent":null,"lastCommit":null,"shortVersion":"wat","authors":[],"owner":null,"versionInfo":{"buildHash":null,"version":{"raw":"wat-release"},"description":"wat-release","package":null},"ref":null,"projects":[{"name":"test","platform":"javascript","slug":"test","platforms":["javascript"],"newGroups":0,"id":1861017}]}"#;

#[test]
fn creates_release() {
    let _server = mock("POST", ENDPOINT)
        .match_body(Matcher::PartialJsonString(
            r#"{"version":"wat-release","projects":["wat-project"]}"#.to_string(),
        ))
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(VALID_RESPONSE)
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .arg("releases")
        .arg("new")
        .arg("wat-release")
        .assert()
        .success()
        .stdout(contains("Created release wat-release."));
}

#[test]
fn allows_for_release_to_start_with_hyphen() {
    let _server = mock("POST", ENDPOINT)
        .match_body(Matcher::PartialJsonString(
            r#"{"version":"-wat-release","projects":["wat-project"]}"#.to_string(),
        ))
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(r#"{"dateReleased":null,"newGroups":0,"commitCount":0,"url":null,"data":{},"lastDeploy":null,"deployCount":0,"dateCreated":"2020-06-29T11:36:59.612687Z","lastEvent":null,"version":"-wat-release","firstEvent":null,"lastCommit":null,"shortVersion":"wat","authors":[],"owner":null,"versionInfo":{"buildHash":null,"version":{"raw":"-wat-release"},"description":"-wat-release","package":null},"ref":null,"projects":[{"name":"test","platform":"javascript","slug":"test","platforms":["javascript"],"newGroups":0,"id":1861017}]}"#)
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .arg("releases")
        .arg("new")
        .arg("-wat-release")
        .assert()
        .success()
        .stdout(contains("Created release -wat-release."));
}

#[test]
fn creates_release_even_if_one_already_exists() {
    let _server = mock("POST", ENDPOINT)
        .match_body(Matcher::PartialJsonString(
            r#"{"version":"wat-release","projects":["wat-project"]}"#.to_string(),
        ))
        .with_status(208)
        .with_header("content-type", "application/json")
        .with_body(VALID_RESPONSE)
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .arg("releases")
        .arg("new")
        .arg("wat-release")
        .assert()
        .success()
        .stdout(contains("Created release wat-release."));
}

#[test]
fn creates_release_with_custom_url() {
    let _server = mock("POST", ENDPOINT)
        .match_body(Matcher::PartialJsonString(
            r#"{"version":"wat-release","projects":["wat-project"],"url":"https://oh.rly"}"#
                .to_string(),
        ))
        .with_status(208)
        .with_header("content-type", "application/json")
        .with_body(VALID_RESPONSE)
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .arg("releases")
        .arg("new")
        .arg("wat-release")
        .arg("--url")
        .arg("https://oh.rly")
        .assert()
        .success()
        .stdout(contains("Created release wat-release."));
}

#[test]
fn creates_release_which_is_instantly_finalized() {
    let _server = mock("POST", ENDPOINT)
        .match_body(Matcher::AllOf(vec![
            Matcher::PartialJsonString(
                r#"{"version":"wat-release","projects":["wat-project"]}"#.to_string(),
            ),
            Matcher::Regex("dateReleased".to_string()),
        ]))
        .with_status(208)
        .with_header("content-type", "application/json")
        .with_body(VALID_RESPONSE)
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .arg("releases")
        .arg("new")
        .arg("wat-release")
        .arg("--finalize")
        .assert()
        .success()
        .stdout(contains("Created release wat-release."));
}
