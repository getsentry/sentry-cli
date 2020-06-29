use mockito::mock;

use assert_cmd::Command;
use predicates::str::{contains, is_match};

use crate::common;

const ENDPOINT: &str = "/api/0/projects/wat-org/wat-project/releases/";
const VALID_RESPONSE: &str = r#"[{"dateReleased":"2020-03-19T10:11:35.128919Z","newGroups":1,"commitCount":0,"url":null,"data":{},"lastDeploy":{"name":null,"url":null,"environment":"x","dateStarted":null,"dateFinished":"2020-05-18T13:39:06.033442Z","id":"6447717"},"deployCount":1,"dateCreated":"2020-03-19T10:11:31.983994Z","lastEvent":null,"version":"vue-1","firstEvent":null,"lastCommit":null,"shortVersion":"vue-1","authors":[],"owner":null,"versionInfo":{"buildHash":null,"version":{"raw":"vue-1"},"description":"vue-1","package":null},"ref":null,"projects":[{"name":"test","platform":"javascript","slug":"test","platforms":["javascript"],"newGroups":1,"id":1861017}]},{"dateReleased":null,"newGroups":0,"commitCount":0,"url":null,"data":{},"lastDeploy":null,"deployCount":0,"dateCreated":"2020-03-16T16:16:12.655209Z","lastEvent":null,"version":"ok","firstEvent":null,"lastCommit":null,"shortVersion":"ok","authors":[],"owner":null,"versionInfo":{"buildHash":null,"version":{"raw":"ok"},"description":"ok","package":null},"ref":null,"projects":[{"name":"test","platform":"javascript","slug":"test","platforms":["javascript"],"newGroups":0,"id":1861017}]}]"#;

#[test]
fn displays_releases() {
    let _server = mock("GET", ENDPOINT)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(VALID_RESPONSE)
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .arg("releases")
        .arg("list")
        .assert()
        .success()
        .stdout(contains(
            "| Released       | Version | New Events | Last Event |",
        ))
        .stdout(
            is_match("\\| \\d+ hours ago \\| vue-1   \\| 1          \\| -          \\|").unwrap(),
        )
        .stdout(contains(
            "| (unreleased)   | ok      | 0          | -          |",
        ));
}

#[test]
fn displays_releases_with_projects() {
    let _server = mock("GET", ENDPOINT)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(VALID_RESPONSE)
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .arg("releases")
        .arg("list")
        .arg("--show-projects")
        .assert()
        .success()
        .stdout(contains(
            "| Released       | Version | Projects | New Events | Last Event |",
        ))
        .stdout(
            is_match(
                "\\| \\d+ hours ago \\| vue-1   \\| test     \\| 1          \\| -          \\|",
            )
            .unwrap(),
        )
        .stdout(contains(
            "| (unreleased)   | ok      | test     | 0          | -          |",
        ));
}

#[test]
fn doesnt_fail_with_empty_response() {
    let _server = mock("GET", ENDPOINT)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .arg("releases")
        .arg("list")
        .assert()
        .success();
}
