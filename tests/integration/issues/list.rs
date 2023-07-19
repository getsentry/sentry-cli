use crate::integration::{mock_endpoint, register_test, EndpointOptions};

#[test]
fn command_issues_list_help() {
    register_test("issues/issues-list-help.trycmd");
}

#[test]
fn doesnt_fail_with_empty_response() {
    let _server = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/issues/?query=&cursor=",
            200,
        )
        .with_response_body("[]"),
    );
    register_test("issues/issues-list-empty.trycmd");
}

#[test]
fn display_issues() {
    let _server = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/issues/?query=&cursor=",
            200,
        )
        .with_response_file("issues/get-issues.json"),
    );
    register_test("issues/issues-display.trycmd");
}

#[test]
fn display_resolved_issues() {
    let _server = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/issues/?query=is:resolved&cursor=",
            200,
        )
        .with_response_file("issues/get-resolved-issues.json"),
    );
    register_test("issues/issues-display-with-query.trycmd");
}
