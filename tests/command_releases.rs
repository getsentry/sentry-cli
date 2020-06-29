use mockito::{mock, server_url};

use assert_cmd::Command;
use predicates::str::contains;
use std::collections::HashMap;

fn get_base_env() -> HashMap<String, String> {
    let mut env = HashMap::new();
    env.insert(String::from("SENTRY_URL"), server_url());
    env.insert(String::from("SENTRY_AUTH_TOKEN"), String::from("lolnope"));
    env.insert(String::from("SENTRY_ORG"), String::from("wat-org"));
    env.insert(String::from("SENTRY_PROJECT"), String::from("wat-project"));
    env
}

#[test]
fn releases_require_subcommand() {
    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(get_base_env())
        .arg("releases")
        .assert()
        .failure()
        .stderr(contains("Manage releases on Sentry."))
        .stderr(contains("sentry-cli releases <SUBCOMMAND>"));
}

#[test]
fn releases_allow_for_overriding_organization_with_flag_for_subcommands() {
    let _server = mock("GET", "/api/0/projects/whynot/wat-project/releases/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create();

    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(get_base_env())
        .arg("releases")
        .arg("--org")
        .arg("whynot")
        .arg("list")
        .assert()
        .success();
}
