use crate::integration::{mock_endpoint, register_test, EndpointOptions};

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
    let _t = register_test("upload_proguard/upload_proguard.trycmd");
}

#[test]
fn command_upload_proguard_upload() {
    let _t = register_test("upload_proguard/upload_proguard-no-upload.trycmd");
}

#[test]
fn command_upload_proguard_reprocessing() {
    let _dsyms = mock_endpoint(
        EndpointOptions::new(
            "POST",
            "/api/0/projects/wat-org/wat-project/files/dsyms/",
            200,
        )
        .with_response_body("[]"),
    );
    let _t = register_test("upload_proguard/upload_proguard-no-reprocessing.trycmd");
}
