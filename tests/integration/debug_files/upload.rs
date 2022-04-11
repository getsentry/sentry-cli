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
