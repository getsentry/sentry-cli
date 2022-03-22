use crate::common::{create_testcase, mock_endpoint, EndpointOptions};

#[test]
fn shows_release_details() {
    let _server = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/wat-release/",
            200,
        )
        .with_response_file("tests/responses/releases/get-release.json"),
    );
    let t = create_testcase();
    t.case("tests/cmd/releases/releases-info.trycmd");
}

#[test]
fn shows_release_details_with_projects_and_commits() {
    let _server = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/wat-release/",
            200,
        )
        .with_response_file("tests/responses/releases/get-release.json"),
    );
    let _commits = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/wat-release/commits/",
            200,
        )
        .with_response_file("tests/responses/releases/get-release-commits.json"),
    );
    let t = create_testcase();
    t.case("tests/cmd/releases/releases-info-with-commits-projects.trycmd");
}

#[test]
fn doesnt_print_output_with_quiet_flag() {
    let _server = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/wat-release/",
            200,
        )
        .with_response_file("tests/responses/releases/get-release.json"),
    );
    let t = create_testcase();
    t.case("tests/cmd/releases/releases-info-quiet.trycmd");
}

#[test]
fn exits_if_no_release_found() {
    let _server = mock_endpoint(EndpointOptions::new(
        "GET",
        "/api/0/projects/wat-org/wat-project/releases/wat-release/",
        404,
    ));
    let t = create_testcase();
    t.case("tests/cmd/releases/releases-info-not-found.trycmd");
}
