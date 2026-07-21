use std::fs;

use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;

use crate::integration::{MockEndpointBuilder, TestManager};

const USER_TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[test]
fn project_url_does_not_use_global_token() {
    let manager = TestManager::new();
    let project_url = manager.server_url();
    // Commands requiring authentication issue no request after the global token is ignored.
    let manager = manager.mock_endpoint(
        MockEndpointBuilder::new("GET", "/api/0/projects/wat-org/wat-project/releases/").expect(0),
    );

    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let project = temp.path().join("project");
    let child = project.join("child");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&child).unwrap();
    fs::write(
        home.join(".sentryclirc"),
        format!("[auth]\ntoken={USER_TOKEN}\n"),
    )
    .unwrap();
    fs::write(
        project.join(".sentryclirc"),
        format!(
            "[defaults]\nurl={project_url}\norg=wat-org\nproject=wat-project\n[http]\nverify_ssl=false\n"
        ),
    )
    .unwrap();

    let mut command = Command::new(cargo_bin!("sentry-cli"));
    let assertion = command
        .env_clear()
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .current_dir(&child)
        .args(["releases", "list"])
        .assert()
        .failure();

    manager.assert_mock_endpoints();

    let stderr = String::from_utf8_lossy(&assertion.get_output().stderr);
    assert!(
        stderr.contains("Ignoring an auth token because the selected URL"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn cli_url_and_environment_token_authenticate_as_one_runtime_source() {
    let manager = TestManager::new();
    let server_url = manager.server_url();
    let manager = manager.mock_endpoint(
        MockEndpointBuilder::new("GET", "/api/0/projects/wat-org/wat-project/releases/")
            .with_header_matcher("authorization", format!("Bearer {USER_TOKEN}").as_str())
            .with_response_body("[]"),
    );

    let mut command = Command::new(cargo_bin!("sentry-cli"));
    command
        .env_clear()
        .env("SENTRY_INTEGRATION_TEST", "1")
        .env("SENTRY_AUTH_TOKEN", USER_TOKEN)
        .env("SENTRY_ORG", "wat-org")
        .env("SENTRY_PROJECT", "wat-project")
        .args(["--url", &server_url, "releases", "list"])
        .assert()
        .success();

    manager.assert_mock_endpoints();
}
