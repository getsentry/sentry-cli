use std::io::{Cursor, Write as _};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use serde_json::json;

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

#[test]
fn command_snapshots_upload_empty_selective_with_inline_names() {
    let snapshots = tempfile::tempdir().unwrap();

    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "POST",
                "/api/0/projects/wat-org/wat-project/preprodartifacts/snapshots/",
            )
            .expect(1)
            .with_response_fn(|request| {
                let compressed = request.body().expect("body should be readable");
                let body = zstd::decode_all(Cursor::new(compressed))
                    .expect("body should be valid zstd data");
                let manifest: serde_json::Value =
                    serde_json::from_slice(&body).expect("body should be valid JSON");

                assert_eq!(manifest["app_id"], "test-app");
                assert_eq!(manifest["images"], json!({}));
                assert_eq!(manifest["selective"], true);
                assert_eq!(
                    manifest["all_image_file_names"],
                    json!(["a.png", "sub/b.jpg"])
                );

                br#"{"artifactId":"snapshot-id","imageCount":0,"snapshotUrl":null}"#.to_vec()
            }),
        )
        .assert_cmd(vec![
            "snapshots",
            "upload",
            snapshots.path().to_str().unwrap(),
            "--app-id",
            "test-app",
            "--all-image-file-names",
            "./a.png,sub\\b.jpg",
            "--no-git-metadata",
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Success);
}

#[test]
fn command_snapshots_upload_empty_selective_with_names_file() {
    let root = tempfile::tempdir().unwrap();
    let snapshots = root.path().join("snapshots");
    let names_file = root.path().join("all-images.txt");
    std::fs::create_dir(&snapshots).unwrap();
    std::fs::write(&names_file, "a.png\nsub/b.png\n").unwrap();

    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "POST",
                "/api/0/projects/wat-org/wat-project/preprodartifacts/snapshots/",
            )
            .expect(1)
            .with_response_fn(|request| {
                let compressed = request.body().expect("body should be readable");
                let body = zstd::decode_all(Cursor::new(compressed))
                    .expect("body should be valid zstd data");
                let manifest: serde_json::Value =
                    serde_json::from_slice(&body).expect("body should be valid JSON");

                assert_eq!(manifest["images"], json!({}));
                assert_eq!(manifest["selective"], true);
                assert_eq!(
                    manifest["all_image_file_names"],
                    json!(["a.png", "sub/b.png"])
                );

                br#"{"artifactId":"snapshot-id","imageCount":0,"snapshotUrl":null}"#.to_vec()
            }),
        )
        .assert_cmd(vec![
            "snapshots".to_owned(),
            "upload".to_owned(),
            snapshots.to_string_lossy().into_owned(),
            "--app-id".to_owned(),
            "test-app".to_owned(),
            "--all-image-file-names-file".to_owned(),
            names_file.to_string_lossy().into_owned(),
            "--no-git-metadata".to_owned(),
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Success);
}

#[test]
fn command_snapshots_upload_empty_selective_without_names_warns() {
    TestManager::new()
        .register_trycmd_test("snapshots/snapshots-upload-empty-selective-without-names.trycmd");
}

#[test]
fn command_snapshots_upload_empty_names_file_fails() {
    let root = tempfile::tempdir().unwrap();
    let snapshots = root.path().join("snapshots");
    let names_file = root.path().join("all-images.txt");
    std::fs::create_dir(&snapshots).unwrap();
    std::fs::write(&names_file, " \n\n").unwrap();

    TestManager::new()
        .assert_cmd(vec![
            "snapshots".to_owned(),
            "upload".to_owned(),
            snapshots.to_string_lossy().into_owned(),
            "--app-id".to_owned(),
            "test-app".to_owned(),
            "--all-image-file-names-file".to_owned(),
            names_file.to_string_lossy().into_owned(),
            "--no-git-metadata".to_owned(),
        ])
        .run_and_assert(AssertCommand::Failure);
}
