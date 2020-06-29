use mockito::{mock, server_url, Matcher};

use assert_cmd::Command;
use predicates::str::contains;
use std::collections::HashMap;

const ENDPOINT: &str = "/api/0/projects/wat-org/wat-project/releases/wat-release/";
const VALID_RESPONSE: &str = r#"{"dateReleased":"2020-06-29T12:16:49.368667Z","newGroups":0,"commitCount":0,"url":null,"data":{},"lastDeploy":null,"deployCount":0,"dateCreated":"2020-06-29T11:36:59.612687Z","lastEvent":null,"version":"wat-release","firstEvent":null,"lastCommit":null,"shortVersion":"wat-release","authors":[],"owner":null,"versionInfo":{"buildHash":null,"version":{"raw":"wat-release"},"description":"wat-release","package":null},"ref":null,"projects":[{"name":"test","platform":"javascript","slug":"test","platforms":["javascript"],"newGroups":0,"id":1861017}]}"#;

fn get_base_env() -> HashMap<String, String> {
    let mut env = HashMap::new();
    env.insert(String::from("SENTRY_URL"), server_url());
    env.insert(String::from("SENTRY_AUTH_TOKEN"), String::from("lolnope"));
    env.insert(String::from("SENTRY_ORG"), String::from("wat-org"));
    env.insert(String::from("SENTRY_PROJECT"), String::from("wat-project"));
    env
}

#[test]
fn finalize_release() {
    let _server = mock("PUT", ENDPOINT)
        .match_body(Matcher::AllOf(vec![
            Matcher::PartialJsonString(r#"{"projects":["wat-project"]}"#.to_string()),
            Matcher::Regex("dateReleased".to_string()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(VALID_RESPONSE)
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(get_base_env())
        .arg("releases")
        .arg("finalize")
        .arg("wat-release")
        .assert()
        .success()
        .stdout(contains("Finalized release wat-release."));
}

#[test]
fn finalize_release_with_custom_dates() {
    let _server = mock("PUT", ENDPOINT)
        .match_body(
            Matcher::JsonString(
                r#"{"projects":["wat-project"],"dateStarted":"2015-05-15T00:01:40Z","dateReleased":"2015-05-15T00:00:00Z"}"#.to_string(),
            ),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(VALID_RESPONSE)
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(get_base_env())
        .arg("releases")
        .arg("finalize")
        .arg("wat-release")
        .arg("--started")
        .arg("1431648100")
        .arg("--released")
        .arg("1431648000")
        .assert()
        .success()
        .stdout(contains("Finalized release wat-release."));
}
