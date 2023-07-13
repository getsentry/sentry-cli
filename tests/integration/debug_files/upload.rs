use crate::integration::{mock_endpoint, register_test, EndpointOptions};

// I have no idea why this is timing out on Windows.
// I verified it manually, and this command works just fine. — Kamil
// TODO: Fix windows timeout.
#[cfg(not(windows))]
#[test]
fn command_debug_files_upload_help() {
    register_test("debug_files/debug_files-upload-help.trycmd");
}

#[test]
fn command_debug_files_upload() {
    let _chunk_upload = mock_endpoint(
        EndpointOptions::new("GET", "/api/0/organizations/wat-org/chunk-upload/", 200)
            .with_response_file("debug_files/get-chunk-upload.json"),
    );
    let _assemble = mock_endpoint(
        EndpointOptions::new(
            "POST",
            "/api/0/projects/wat-org/wat-project/files/difs/assemble/",
            200,
        )
        .with_response_file("debug_files/post-difs-assemble.json"),
    );
    let _reprocessing = mock_endpoint(
        EndpointOptions::new(
            "POST",
            "/api/0/projects/wat-org/wat-project/reprocessing/",
            200,
        )
        .with_response_body("[]"),
    );
    register_test("debug_files/debug_files-upload.trycmd");
}

#[test]
fn command_debug_files_upload_pdb() {
    let _chunk_upload = mock_endpoint(
        EndpointOptions::new("GET", "/api/0/organizations/wat-org/chunk-upload/", 200)
            .with_response_file("debug_files/get-chunk-upload.json"),
    );
    let _assemble = mock_endpoint(
        EndpointOptions::new(
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
    let _reprocessing = mock_endpoint(
        EndpointOptions::new(
            "POST",
            "/api/0/projects/wat-org/wat-project/reprocessing/",
            200,
        )
        .with_response_body("[]"),
    );
    register_test("debug_files/debug_files-upload-pdb.trycmd");
    register_test("debug_files/debug_files-upload-pdb-include-sources.trycmd");
}

#[test]
fn command_debug_files_upload_pdb_embedded_sources() {
    let _chunk_upload = mock_endpoint(
        EndpointOptions::new("GET", "/api/0/organizations/wat-org/chunk-upload/", 200)
            .with_response_file("debug_files/get-chunk-upload.json"),
    );
    let _assemble = mock_endpoint(
        EndpointOptions::new(
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
    let _reprocessing = mock_endpoint(
        EndpointOptions::new(
            "POST",
            "/api/0/projects/wat-org/wat-project/reprocessing/",
            200,
        )
        .with_response_body("[]"),
    );
    register_test("debug_files/debug_files-upload-pdb-embedded-sources.trycmd");
}

#[test]
fn command_debug_files_upload_dll_embedded_ppdb_with_sources() {
    let _chunk_upload = mock_endpoint(
        EndpointOptions::new("GET", "/api/0/organizations/wat-org/chunk-upload/", 200)
            .with_response_file("debug_files/get-chunk-upload.json"),
    );
    let _assemble = mock_endpoint(
        EndpointOptions::new(
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
    let _reprocessing = mock_endpoint(
        EndpointOptions::new(
            "POST",
            "/api/0/projects/wat-org/wat-project/reprocessing/",
            200,
        )
        .with_response_body("[]"),
    );
    register_test("debug_files/debug_files-upload-dll-embedded-ppdb-with-sources.trycmd");
}

#[test]
fn command_debug_files_upload_mixed_embedded_sources() {
    let _chunk_upload = mock_endpoint(
        EndpointOptions::new("GET", "/api/0/organizations/wat-org/chunk-upload/", 200)
            .with_response_file("debug_files/get-chunk-upload.json"),
    );
    let _assemble = mock_endpoint(
        EndpointOptions::new(
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
    let _reprocessing = mock_endpoint(
        EndpointOptions::new(
            "POST",
            "/api/0/projects/wat-org/wat-project/reprocessing/",
            200,
        )
        .with_response_body("[]"),
    );
    // TODO this isn't tested properly at the moment, because `indicatif` ProgressBar (at least at the current version)
    //      swallows debug logs printed while the progress bar is active and the session is not attended.
    //      See how it's supposed to look like `debug_files-bundle_sources-mixed-embedded-sources.trycmd` and try it out
    //      after an update of `indicatif` to the latest version (currently it's blocked by some other issues).
    register_test("debug_files/debug_files-upload-mixed-embedded-sources.trycmd");
}

#[test]
fn command_debug_files_upload_no_upload() {
    let _chunk_upload = mock_endpoint(
        EndpointOptions::new("GET", "/api/0/organizations/wat-org/chunk-upload/", 200)
            .with_response_file("debug_files/get-chunk-upload.json"),
    );
    let _assemble = mock_endpoint(
        EndpointOptions::new(
            "POST",
            "/api/0/projects/wat-org/wat-project/files/difs/assemble/",
            200,
        )
        .with_response_file("debug_files/post-difs-assemble.json"),
    );
    register_test("debug_files/debug_files-upload-no-upload.trycmd");
}

#[test]
fn command_debug_files_upload_no_reprocessing() {
    let _chunk_upload = mock_endpoint(
        EndpointOptions::new("GET", "/api/0/organizations/wat-org/chunk-upload/", 200)
            .with_response_file("debug_files/get-chunk-upload.json"),
    );
    let _assemble = mock_endpoint(
        EndpointOptions::new(
            "POST",
            "/api/0/projects/wat-org/wat-project/files/difs/assemble/",
            200,
        )
        .with_response_file("debug_files/post-difs-assemble.json"),
    );
    register_test("debug_files/debug_files-upload-no-reprocessing.trycmd");
}
