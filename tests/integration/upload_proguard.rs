use crate::integration::{
    mock_endpoint, register_test, register_test_without_token, EndpointOptions,
};

#[test]
fn command_upload_proguard_help() {
    register_test("upload_proguard/upload_proguard-help.trycmd");
}

#[test]
fn command_upload_proguard() {
    let _dsyms = mock_endpoint(
        EndpointOptions::new(
            "POST",
            "/api/0/projects/wat-org/wat-project/files/dsyms/",
            200,
        )
        .with_response_body("[]"),
    );
    let _reprocessing = mock_endpoint(
        EndpointOptions::new(
            "POST",
            "/api/0/projects/wat-org/wat-project/reprocessing/",
            200,
        )
        .with_response_body("[]"),
    );
    register_test("upload_proguard/upload_proguard.trycmd");
}

#[test]
fn command_upload_proguard_no_upload() {
    register_test("upload_proguard/upload_proguard-no-upload.trycmd");
}

#[test]
fn command_upload_proguard_no_upload_no_auth_token() {
    register_test_without_token("upload_proguard/upload_proguard-no-upload.trycmd");
}

#[test]
fn command_upload_proguard_no_reprocessing() {
    let _dsyms = mock_endpoint(
        EndpointOptions::new(
            "POST",
            "/api/0/projects/wat-org/wat-project/files/dsyms/",
            200,
        )
        .with_response_body("[]"),
    );
    register_test("upload_proguard/upload_proguard-no-reprocessing.trycmd");
}
