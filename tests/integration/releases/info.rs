use crate::integration::{mock_endpoint, register_test, MockEndpointBuilder};

#[test]
fn shows_release_details() {
    let _server = mock_endpoint(
        MockEndpointBuilder::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/wat-release/",
            200,
        )
        .with_response_file("releases/get-release.json"),
    );
    register_test("releases/releases-info.trycmd");
}

#[test]
fn shows_release_details_with_projects_and_commits() {
    let _server = mock_endpoint(
        MockEndpointBuilder::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/wat-release/",
            200,
        )
        .with_response_file("releases/get-release.json"),
    );
    let _commits = mock_endpoint(
        MockEndpointBuilder::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/wat-release/commits/",
            200,
        )
        .with_response_file("releases/get-release-commits.json"),
    );
    register_test("releases/releases-info-with-commits-projects.trycmd");
}

#[test]
fn doesnt_print_output_with_quiet_flag() {
    let _server = mock_endpoint(
        MockEndpointBuilder::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/wat-release/",
            200,
        )
        .with_response_file("releases/get-release.json"),
    );
    register_test("releases/releases-info-quiet.trycmd");
}

#[test]
fn doesnt_print_output_with_silent_flag() {
    let _server = mock_endpoint(
        MockEndpointBuilder::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/wat-release/",
            200,
        )
        .with_response_file("releases/get-release.json"),
    );
    register_test("releases/releases-info-silent.trycmd");
}

#[test]
fn preserve_valid_exit_code_with_quiet_flag() {
    let _server = mock_endpoint(MockEndpointBuilder::new(
        "GET",
        "/api/0/projects/wat-org/wat-project/releases/unknown-release/",
        404,
    ));
    register_test("releases/releases-info-quiet-failed.trycmd");
}

#[test]
fn exits_if_no_release_found() {
    let _server = mock_endpoint(MockEndpointBuilder::new(
        "GET",
        "/api/0/projects/wat-org/wat-project/releases/wat-release/",
        404,
    ));
    register_test("releases/releases-info-not-found.trycmd");
}
