use assert_cmd::Command;

use crate::integration::{mock_endpoint, register_test, test_utils::env, MockEndpointBuilder};

#[test]
fn command_debug_files_upload() {
    let _chunk_upload = mock_endpoint(
        MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/", 200)
            .with_response_file("debug_files/get-chunk-upload.json"),
    );
    let _assemble = mock_endpoint(
        MockEndpointBuilder::new(
            "POST",
            "/api/0/projects/wat-org/wat-project/files/difs/assemble/",
            200,
        )
        .with_response_file("debug_files/post-difs-assemble.json"),
    );
    register_test("debug_files/upload/debug_files-upload.trycmd");
}

#[test]
fn command_debug_files_upload_pdb() {
    let _chunk_upload = mock_endpoint(
        MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/", 200)
            .with_response_file("debug_files/get-chunk-upload.json"),
    );
    let _assemble = mock_endpoint(
        MockEndpointBuilder::new(
            "POST",
            "/api/0/projects/wat-org/wat-project/files/difs/assemble/",
            200,
        )
        .with_response_body(
            r#"{
                "5f81d6becc51980870acc9f6636ab53d26160763": {
                    "state": "ok",
                    "missingChunks": []
                }
            }"#,
        ),
    );
    register_test("debug_files/upload/debug_files-upload-pdb.trycmd");
    register_test("debug_files/upload/debug_files-upload-pdb-include-sources.trycmd");
}

#[test]
fn command_debug_files_upload_pdb_embedded_sources() {
    let _chunk_upload = mock_endpoint(
        MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/", 200)
            .with_response_file("debug_files/get-chunk-upload.json"),
    );
    let _assemble = mock_endpoint(
        MockEndpointBuilder::new(
            "POST",
            "/api/0/projects/wat-org/wat-project/files/difs/assemble/",
            200,
        )
        .with_response_body(
            r#"{
                "50dd9456dc89cdbc767337da512bdb36b15db6b2": {
                    "state": "ok",
                    "missingChunks": []
                }
            }"#,
        ),
    );
    register_test("debug_files/upload/debug_files-upload-pdb-embedded-sources.trycmd");
}

#[test]
fn command_debug_files_upload_dll_embedded_ppdb_with_sources() {
    let _chunk_upload = mock_endpoint(
        MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/", 200)
            .with_response_file("debug_files/get-chunk-upload.json"),
    );
    let _assemble = mock_endpoint(
        MockEndpointBuilder::new(
            "POST",
            "/api/0/projects/wat-org/wat-project/files/difs/assemble/",
            200,
        )
        .with_response_body(
            r#"{
                "fc1c9e58a65bd4eaf973bbb7e7a7cc01bfdaf15e": {
                    "state": "ok",
                    "missingChunks": []
                }
            }"#,
        ),
    );
    register_test("debug_files/upload/debug_files-upload-dll-embedded-ppdb-with-sources.trycmd");
}

#[test]
fn command_debug_files_upload_mixed_embedded_sources() {
    let _chunk_upload = mock_endpoint(
        MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/", 200)
            .with_response_file("debug_files/get-chunk-upload.json"),
    );
    let _assemble = mock_endpoint(
        MockEndpointBuilder::new(
            "POST",
            "/api/0/projects/wat-org/wat-project/files/difs/assemble/",
            200,
        )
        .with_response_body(
            r#"{
                "21b76b717dbbd8c89e42d92b29667ac87aa3c124": {
                    "state": "ok",
                    "missingChunks": []
                }
            }"#,
        ),
    );
    // TODO this isn't tested properly at the moment, because `indicatif` ProgressBar (at least at the current version)
    //      swallows debug logs printed while the progress bar is active and the session is not attended.
    //      See how it's supposed to look like `debug_files-bundle_sources-mixed-embedded-sources.trycmd` and try it out
    //      after an update of `indicatif` to the latest version (currently it's blocked by some other issues).
    register_test("debug_files/upload/debug_files-upload-mixed-embedded-sources.trycmd");
}

#[test]
fn command_debug_files_upload_no_upload() {
    let _chunk_upload = mock_endpoint(
        MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/", 200)
            .with_response_file("debug_files/get-chunk-upload.json"),
    );
    let _assemble = mock_endpoint(
        MockEndpointBuilder::new(
            "POST",
            "/api/0/projects/wat-org/wat-project/files/difs/assemble/",
            200,
        )
        .with_response_file("debug_files/post-difs-assemble.json"),
    );
    register_test("debug_files/upload/debug_files-upload-no-upload.trycmd");
}

#[test]
/// This test ensures that the correct initial call to the debug files assemble endpoint is made.
/// The mock assemble endpoint returns a 200 response simulating the case where all chunks
/// are already uploaded.
fn ensure_correct_assemble_call() {
    let _chunk_upload = mock_endpoint(
        MockEndpointBuilder::new("GET", "/api/0/organizations/wat-org/chunk-upload/", 200)
            .with_response_file("debug_files/get-chunk-upload.json"),
    );

    let assemble = mockito::mock("POST", "/api/0/projects/wat-org/wat-project/files/difs/assemble/")
        .match_body(r#"{"21b76b717dbbd8c89e42d92b29667ac87aa3c124":{"name":"SrcGenSampleApp.pdb","debug_id":"c02651ae-cd6f-492d-bc33-0b83111e7106-8d8e7c60","chunks":["21b76b717dbbd8c89e42d92b29667ac87aa3c124"]}}"#)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
                "21b76b717dbbd8c89e42d92b29667ac87aa3c124": {
                    "state": "ok",
                    "missingChunks": []
                }
            }"#)
        .create();

    let mut command = Command::cargo_bin("sentry-cli").expect("sentry-cli should be available");

    command.args(
        "debug-files upload --include-sources tests/integration/_fixtures/SrcGenSampleApp.pdb"
            .split(' '),
    );

    env::set_all(|k, v| {
        command.env(k, v.as_ref());
    });

    command.env(
        "SENTRY_AUTH_TOKEN",
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    );

    let command_result = command.assert();

    // First assert the mock was called as expected, then that the command was successful.
    // This is because failure with the mock assertion can cause the command to fail, and
    // the mock assertion failure is likely more interesting in this case.
    assemble.assert();
    command_result.success();
}
