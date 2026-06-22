use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_snapshots_diff_help() {
    TestManager::new().register_trycmd_test("snapshots/snapshots-diff-help.trycmd");
}

#[test]
fn command_snapshots_diff_missing_dir() {
    TestManager::new().register_trycmd_test("snapshots/snapshots-diff-missing-dir.trycmd");
}

#[test]
fn command_snapshots_download_help() {
    TestManager::new().register_trycmd_test("snapshots/snapshots-download-help.trycmd");
}

#[test]
fn command_snapshots_upload_help() {
    TestManager::new().register_trycmd_test("snapshots/snapshots-upload-help.trycmd");
}

#[test]
fn command_snapshots_upload_renamed_project() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/preprodartifacts/snapshots/upload-options/",
            )
            .with_status(302)
            .with_response_body(
                r#"{"slug":"new-project-slug","detail":{"extra":{"url":"/api/0/projects/wat-org/new-project-slug/preprodartifacts/snapshots/upload-options/","slug":"new-project-slug"}}}"#,
            ),
        )
        .register_trycmd_test("snapshots/snapshots-upload-renamed-project.trycmd")
        .with_default_token();
}
