use std::io::{Cursor, Write as _};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use serde_json::json;
use sha2::{Digest as _, Sha256};

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
                "/api/0/projects/wat-org/wat-project/preprodartifacts/snapshots/upload-options/?usecase=auto",
            )
            .with_status(302)
            .with_response_body(
                r#"{"slug":"new-project-slug","detail":{"extra":{"url":"/api/0/projects/wat-org/new-project-slug/preprodartifacts/snapshots/upload-options/","slug":"new-project-slug"}}}"#,
            ),
        )
        .register_trycmd_test("snapshots/snapshots-upload-renamed-project.trycmd")
        .with_default_token();
}

#[rstest::rstest]
#[case::preprod_snapshots(Some("preprod_snapshots"), "preprod_snapshots")]
#[case::preprod(Some("preprod"), "preprod")]
#[case::legacy(None, "preprod")]
fn command_snapshots_upload_uses_server_usecase(
    #[case] returned_usecase: Option<&str>,
    #[case] expected_usecase: &str,
) {
    let mut objectstore = mockito::Server::new();
    let image = std::fs::read("tests/integration/_fixtures/snapshots/snapshot.png").unwrap();
    let hash = format!("{:x}", Sha256::digest(image));
    let batch_path = format!("/proxy/v1/objects:batch/{expected_usecase}/org=1;project=2/");
    let objectstore_mocks: Vec<_> = [("head", 404), ("insert", 200)]
        .into_iter()
        .map(|(operation, status)| {
            objectstore
                .mock("POST", batch_path.as_str())
                .match_header("x-os-auth", "Bearer objectstore-token")
                .match_body(mockito::Matcher::AllOf(vec![
                    mockito::Matcher::Regex(format!(
                        "x-sn-batch-operation-kind: {operation}\\r\\n"
                    )),
                    mockito::Matcher::Regex(format!(
                        "x-sn-batch-operation-key: 1%2F2%2F{hash}\\r\\n"
                    )),
                ]))
                .with_header("content-type", "multipart/form-data; boundary=response")
                .with_body(format!(
                    "--response\r\n\
                     Content-Disposition: form-data; name=\"part\"\r\n\
                     x-sn-batch-operation-index: 0\r\n\
                     x-sn-batch-operation-status: {status}\r\n\
                     \r\n\r\n--response--\r\n"
                ))
                .expect(1)
                .create()
        })
        .collect();
    let mut upload_options = json!({
        "objectstore": {
            "url": format!("{}/proxy", objectstore.url()),
            "scopes": [["org", "1"], ["project", "2"]],
            "authToken": "objectstore-token",
            "expirationPolicy": "tti:30d"
        }
    });
    if let Some(usecase) = returned_usecase {
        upload_options["objectstore"]["usecase"] = json!(usecase);
    }

    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/preprodartifacts/snapshots/upload-options/?usecase=auto",
            )
            .expect(1)
            .with_response_body(upload_options.to_string()),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "POST",
                "/api/0/projects/wat-org/wat-project/preprodartifacts/snapshots/",
            )
            .expect(1)
            .with_response_body(r#"{"artifactId":"snapshot-id","imageCount":1,"snapshotUrl":null}"#),
        )
        .assert_cmd(vec![
            "snapshots",
            "upload",
            "tests/integration/_fixtures/snapshots",
            "--app-id",
            "test-app",
            "--no-git-metadata",
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Success);

    for mock in objectstore_mocks {
        mock.assert();
    }
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
