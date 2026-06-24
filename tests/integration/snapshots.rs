use std::io::{Cursor, Write as _};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::integration::{AssertCommand, MockEndpointBuilder, TestManager};

fn snapshot_zip_bytes() -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file("snapshot.png", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"fake png bytes").unwrap();
    zip.finish().unwrap().into_inner()
}

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
fn command_snapshots_download_ready() {
    let output = tempfile::tempdir().unwrap();
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/organizations/wat-org/preprodartifacts/snapshots/123/archive/",
            )
            .with_response_body(r#"{"ready":true}"#),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/organizations/wat-org/preprodartifacts/snapshots/123/archive/?download",
            )
            .with_response_body(snapshot_zip_bytes()),
        )
        .assert_cmd(vec![
            "snapshots",
            "download",
            "--org",
            "wat-org",
            "--snapshot-id",
            "123",
            "--output",
            output.path().to_str().unwrap(),
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Success);
}

#[test]
fn command_snapshots_download_builds_then_downloads() {
    let output = tempfile::tempdir().unwrap();
    let probe_count = Arc::new(AtomicUsize::new(0));
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/organizations/wat-org/preprodartifacts/snapshots/123/archive/",
            )
            .expect(2)
            .with_response_fn(move |_| {
                if probe_count.fetch_add(1, Ordering::SeqCst) == 0 {
                    br#"{"ready":false}"#.to_vec()
                } else {
                    br#"{"ready":true}"#.to_vec()
                }
            }),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "POST",
                "/api/0/organizations/wat-org/preprodartifacts/snapshots/123/archive/",
            )
            .with_status(202)
            .with_response_body(r#"{"detail":"Building your snapshot archive."}"#),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/organizations/wat-org/preprodartifacts/snapshots/123/archive/?download",
            )
            .with_response_body(snapshot_zip_bytes()),
        )
        .assert_cmd(vec![
            "snapshots",
            "download",
            "--org",
            "wat-org",
            "--snapshot-id",
            "123",
            "--output",
            output.path().to_str().unwrap(),
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Success);
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
