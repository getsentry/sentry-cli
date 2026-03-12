use std::sync::atomic::{AtomicU16, Ordering};

use crate::integration::{AssertCommand, MockEndpointBuilder, TestManager};

#[test]
fn command_code_mappings_upload() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("POST", "/api/0/organizations/wat-org/code-mappings/bulk/")
                .with_response_file("code_mappings/post-bulk.json"),
        )
        .register_trycmd_test("code_mappings/code-mappings-upload.trycmd")
        .with_default_token();
}

#[test]
fn command_code_mappings_upload_partial_error() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("POST", "/api/0/organizations/wat-org/code-mappings/bulk/")
                .with_response_file("code_mappings/post-bulk-partial-error.json"),
        )
        .register_trycmd_test("code_mappings/code-mappings-upload-partial-error.trycmd")
        .with_default_token();
}

#[test]
fn command_code_mappings_upload_batches() {
    // Generate a fixture with 301 mappings to force 2 batches (300 + 1).
    let mut mappings = Vec::with_capacity(301);
    for i in 0..301 {
        mappings.push(serde_json::json!({
            "stackRoot": format!("com/example/m{i}"),
            "sourceRoot": format!("modules/m{i}/src/main/java/com/example/m{i}"),
        }));
    }
    let fixture = tempfile::NamedTempFile::new().expect("failed to create temp file");
    serde_json::to_writer(&fixture, &mappings).expect("failed to write fixture");

    let call_count = AtomicU16::new(0);

    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new("POST", "/api/0/organizations/wat-org/code-mappings/bulk/")
                .expect(2)
                .with_response_fn(move |_request| {
                    let n = call_count.fetch_add(1, Ordering::Relaxed);
                    // Return appropriate counts per batch
                    let (created, mapping_count) = if n == 0 { (300, 300) } else { (1, 1) };
                    let mut batch_mappings = Vec::new();
                    for i in 0..mapping_count {
                        let idx = n as usize * 300 + i;
                        batch_mappings.push(serde_json::json!({
                        "stackRoot": format!("com/example/m{idx}"),
                        "sourceRoot": format!("modules/m{idx}/src/main/java/com/example/m{idx}"),
                        "status": "created",
                    }));
                    }
                    serde_json::to_vec(&serde_json::json!({
                        "created": created,
                        "updated": 0,
                        "errors": 0,
                        "mappings": batch_mappings,
                    }))
                    .expect("failed to serialize response")
                }),
        )
        .assert_cmd([
            "code-mappings",
            "upload",
            fixture.path().to_str().expect("valid utf-8 path"),
            "--org",
            "wat-org",
            "--project",
            "wat-project",
            "--repo",
            "owner/repo",
            "--default-branch",
            "main",
        ])
        .with_default_token()
        .run_and_assert(AssertCommand::Success);
}
