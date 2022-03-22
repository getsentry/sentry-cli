use crate::common::{create_testcase, mock_endpoint, EndpointOptions};

#[test]
fn successfully_deletes() {
    let _server = mock_endpoint(EndpointOptions::new(
        "DELETE",
        "/api/0/projects/wat-org/wat-project/releases/wat-release/",
        204,
    ));
    let t = create_testcase();
    t.case("tests/cmd/releases/releases-delete.trycmd");
}

#[test]
fn allows_for_release_to_start_with_hyphen() {
    let _server = mock_endpoint(EndpointOptions::new(
        "DELETE",
        "/api/0/projects/wat-org/wat-project/releases/-wat-release/",
        204,
    ));
    let t = create_testcase();
    t.case("tests/cmd/releases/releases-delete-hyphen.trycmd");
}

#[test]
fn informs_about_nonexisting_releases() {
    let _server = mock_endpoint(EndpointOptions::new(
        "DELETE",
        "/api/0/projects/wat-org/wat-project/releases/whoops/",
        404,
    ));
    let t = create_testcase();
    t.case("tests/cmd/releases/releases-delete-nonexisting.trycmd");
}

#[test]
fn doesnt_allow_to_delete_active_releases() {
    let _server = mock_endpoint(
        EndpointOptions::new(
            "DELETE",
            "/api/0/projects/wat-org/wat-project/releases/wat-release/",
            400,
        )
        .with_response_file("tests/responses/releases/delete-active-release.json"),
    );
    let t = create_testcase();
    t.case("tests/cmd/releases/releases-delete-active.trycmd");
}
