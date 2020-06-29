use assert_cmd::Command;
use mockito::{mock, Matcher};
use predicates::str::contains;

use crate::common;

#[test]
fn successfully_creates_a_release() {
    let _server = mock(
        "PUT",
        "/api/0/projects/wat-org/wat-project/releases/wat-release/",
    )
    .match_body(Matcher::AllOf(vec![
        Matcher::PartialJsonString(r#"{"projects":["wat-project"]}"#.to_string()),
        Matcher::Regex("dateReleased".to_string()),
    ]))
    .with_status(200)
    .with_header("content-type", "application/json")
    .with_body(r#"{"dateReleased":"2020-06-29T12:16:49.368667Z","newGroups":0,"commitCount":0,"url":null,"data":{},"lastDeploy":null,"deployCount":0,"dateCreated":"2020-06-29T11:36:59.612687Z","lastEvent":null,"version":"wat-release","firstEvent":null,"lastCommit":null,"shortVersion":"wat-release","authors":[],"owner":null,"versionInfo":{"buildHash":null,"version":{"raw":"wat-release"},"description":"wat-release","package":null},"ref":null,"projects":[{"name":"test","platform":"javascript","slug":"test","platforms":["javascript"],"newGroups":0,"id":1861017}]}"#)
    .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .args(vec!["releases", "finalize", "wat-release"])
        .assert()
        .success()
        .stdout(contains("Finalized release wat-release."));
}

#[test]
fn allows_for_release_to_start_with_hyphen() {
    let _server = mock("PUT", "/api/0/projects/wat-org/wat-project/releases/-wat-release/")
        .match_body(Matcher::AllOf(vec![
            Matcher::PartialJsonString(r#"{"projects":["wat-project"]}"#.to_string()),
            Matcher::Regex("dateReleased".to_string()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"dateReleased":"2020-06-29T12:16:49.368667Z","newGroups":0,"commitCount":0,"url":null,"data":{},"lastDeploy":null,"deployCount":0,"dateCreated":"2020-06-29T11:36:59.612687Z","lastEvent":null,"version":"-wat-release","firstEvent":null,"lastCommit":null,"shortVersion":"-wat-release","authors":[],"owner":null,"versionInfo":{"buildHash":null,"version":{"raw":"-wat-release"},"description":"-wat-release","package":null},"ref":null,"projects":[{"name":"test","platform":"javascript","slug":"test","platforms":["javascript"],"newGroups":0,"id":1861017}]}"#)
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .args(vec!["releases", "finalize", "-wat-release"])
        .assert()
        .success()
        .stdout(contains("Finalized release -wat-release."));
}

#[test]
fn release_with_custom_dates() {
    let _server = mock("PUT", "/api/0/projects/wat-org/wat-project/releases/wat-release/")
        .match_body(
            Matcher::JsonString(
                r#"{"projects":["wat-project"],"dateStarted":"2015-05-15T00:01:40Z","dateReleased":"2015-05-15T00:00:00Z"}"#.to_string(),
            ),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"dateReleased":"2020-06-29T12:16:49.368667Z","newGroups":0,"commitCount":0,"url":null,"data":{},"lastDeploy":null,"deployCount":0,"dateCreated":"2020-06-29T11:36:59.612687Z","lastEvent":null,"version":"wat-release","firstEvent":null,"lastCommit":null,"shortVersion":"wat-release","authors":[],"owner":null,"versionInfo":{"buildHash":null,"version":{"raw":"wat-release"},"description":"wat-release","package":null},"ref":null,"projects":[{"name":"test","platform":"javascript","slug":"test","platforms":["javascript"],"newGroups":0,"id":1861017}]}"#)
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .args(vec![
            "releases",
            "finalize",
            "wat-release",
            "--started",
            "1431648100",
            "--released",
            "1431648000",
        ])
        .assert()
        .success()
        .stdout(contains("Finalized release wat-release."));
}
