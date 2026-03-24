use crate::integration::{AssertCommand, MockEndpointBuilder, TestManager};

#[test]
fn command_build_download_help() {
    TestManager::new().register_trycmd_test("build/build-download-help.trycmd");
}

#[test]
fn command_build_download_not_installable() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/organizations/wat-org/preprodartifacts/123/install-details/",
            )
            .with_response_body(r#"{"isInstallable": false, "installUrl": null}"#),
        )
        .assert_cmd(vec!["build", "download", "--build-id", "123"])
        .with_default_token()
        .run_and_assert(AssertCommand::Failure);
}

#[test]
fn command_build_download_apk() {
    let manager = TestManager::new();
    let server_url = manager.server_url();
    let download_path = format!("{server_url}/download/build.apk?response_format=apk");
    let install_details_response = serde_json::json!({
        "isInstallable": true,
        "installUrl": download_path,
    })
    .to_string();

    let output = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    let output_path = output.path().to_str().unwrap().to_owned();

    manager
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/organizations/wat-org/preprodartifacts/456/install-details/",
            )
            .with_response_body(install_details_response),
        )
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/download/build.apk?response_format=apk")
                .with_response_body("fake apk content"),
        )
        .assert_cmd(vec![
            "build",
            "download",
            "--build-id",
            "456",
            "--output",
            &output_path,
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Success);

    let content = std::fs::read_to_string(&output_path).expect("Failed to read downloaded file");
    assert_eq!(content, "fake apk content");
}

#[test]
fn command_build_download_ipa_converts_plist_format() {
    let manager = TestManager::new();
    let server_url = manager.server_url();
    // The install URL uses plist format, which should be converted to ipa
    let install_url = format!("{server_url}/download/build.ipa?response_format=plist");
    let install_details_response = serde_json::json!({
        "isInstallable": true,
        "installUrl": install_url,
    })
    .to_string();

    let output = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    let output_path = output.path().to_str().unwrap().to_owned();

    manager
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/organizations/wat-org/preprodartifacts/789/install-details/",
            )
            .with_response_body(install_details_response),
        )
        // The mock should receive the converted URL with response_format=ipa
        .mock_endpoint(
            MockEndpointBuilder::new("GET", "/download/build.ipa?response_format=ipa")
                .with_response_body("fake ipa content"),
        )
        .assert_cmd(vec![
            "build",
            "download",
            "--build-id",
            "789",
            "--output",
            &output_path,
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Success);

    let content = std::fs::read_to_string(&output_path).expect("Failed to read downloaded file");
    assert_eq!(content, "fake ipa content");
}

#[test]
fn command_build_download_unsupported_format() {
    let manager = TestManager::new();
    let server_url = manager.server_url();
    let download_path = format!("{server_url}/download/build.zip?response_format=zip");
    let install_details_response = serde_json::json!({
        "isInstallable": true,
        "installUrl": download_path,
    })
    .to_string();

    manager
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/organizations/wat-org/preprodartifacts/999/install-details/",
            )
            .with_response_body(install_details_response),
        )
        .assert_cmd(vec!["build", "download", "--build-id", "999"])
        .with_default_token()
        .run_and_assert(AssertCommand::Failure);
}
