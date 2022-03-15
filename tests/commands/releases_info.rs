use assert_cmd::Command;
use mockito::mock;
use predicates::prelude::*;
use predicates::str::{contains, is_empty};

use crate::common;

#[test]
fn shows_release_details() {
    let _server = mock("GET", "/api/0/projects/wat-org/wat-project/releases/wat-release/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"dateReleased":"2020-06-29T12:16:49.368667Z","newGroups":0,"commitCount":0,"url":null,"data":{},"lastDeploy":null,"deployCount":0,"dateCreated":"2020-06-29T11:36:59.612687Z","lastEvent":null,"version":"wat-release","firstEvent":null,"lastCommit":null,"shortVersion":"wat-release","authors":[],"owner":null,"versionInfo":{"buildHash":null,"version":{"raw":"wat-release"},"description":"wat-release","package":null},"ref":null,"projects":[{"name":"test","platform":"javascript","slug":"test","platforms":["javascript"],"newGroups":0,"id":1861017}]}"#)
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .args(vec!["releases", "info", "wat-release"])
        .assert()
        .success()
        .stdout(
            contains("| Version     | Date created                   |")
                .and(contains("| wat-release | 2020-06-29 11:36:59.612687 UTC |")),
        );
}

#[test]
fn shows_release_details_with_projects_and_commits() {
    let _server = mock("GET", "/api/0/projects/wat-org/wat-project/releases/wat-release/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"dateReleased":"2020-06-29T12:16:49.368667Z","newGroups":0,"commitCount":0,"url":null,"data":{},"lastDeploy":null,"deployCount":0,"dateCreated":"2020-06-29T11:36:59.612687Z","lastEvent":null,"version":"wat-release","firstEvent":null,"lastCommit":null,"shortVersion":"wat-release","authors":[],"owner":null,"versionInfo":{"buildHash":null,"version":{"raw":"wat-release"},"description":"wat-release","package":null},"ref":null,"projects":[{"name":"test","platform":"javascript","slug":"test","platforms":["javascript"],"newGroups":0,"id":1861017}]}"#)
        .create();

    let _commits = mock(
        "GET",
        "/api/0/projects/wat-org/wat-project/releases/wat-release/commits/",
    )
    .with_status(200)
    .with_header("content-type", "application/json")
    .with_body(r#"[{"id":"iddqd"},{"id":"idkfa"}]"#)
    .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .args(vec![
            "releases",
            "info",
            "wat-release",
            "--show-commits",
            "--show-projects",
        ])
        .assert()
        .success()
        .stdout(
            contains("| Version     | Date created                   | Projects | Commits |")
                .and(contains(
                    "| wat-release | 2020-06-29 11:36:59.612687 UTC | test     | iddqd   |",
                ))
                .and(contains(
                    "|             |                                |          | idkfa   |",
                )),
        );
}

#[test]
fn doesnt_print_output_with_quiet_flag() {
    let _server = mock("GET", "/api/0/projects/wat-org/wat-project/releases/wat-release/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"dateReleased":"2020-06-29T12:16:49.368667Z","newGroups":0,"commitCount":0,"url":null,"data":{},"lastDeploy":null,"deployCount":0,"dateCreated":"2020-06-29T11:36:59.612687Z","lastEvent":null,"version":"wat-release","firstEvent":null,"lastCommit":null,"shortVersion":"wat-release","authors":[],"owner":null,"versionInfo":{"buildHash":null,"version":{"raw":"wat-release"},"description":"wat-release","package":null},"ref":null,"projects":[{"name":"test","platform":"javascript","slug":"test","platforms":["javascript"],"newGroups":0,"id":1861017}]}"#)
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .args(vec!["releases", "info", "wat-release", "--quiet"])
        .assert()
        .success()
        .stdout(is_empty());
}

#[test]
fn exits_if_no_release_found() {
    let _server = mock(
        "GET",
        "/api/0/projects/wat-org/wat-project/releases/wat-release/",
    )
    .with_status(404)
    .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
        .args(vec!["releases", "info", "wat-release"])
        .assert()
        .failure();
}
