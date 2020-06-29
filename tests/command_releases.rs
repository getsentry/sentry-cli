use mockito::mock;

use assert_cmd::Command;
use predicates::str::contains;

mod common;

#[test]
fn releases_require_subcommand() {
    Command::cargo_bin("sentry-cli")
        .unwrap()
        .envs(common::get_base_env())
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
        .envs(common::get_base_env())
        .arg("releases")
        .arg("--org")
        .arg("whynot")
        .arg("list")
        .assert()
        .success();
}
