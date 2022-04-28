use crate::integration::{mock_endpoint, register_test, EndpointOptions};

#[test]
fn command_projects_list() {
    let _server = mock_endpoint(
        EndpointOptions::new("GET", "/api/0/organizations/wat-org/projects/?cursor=", 200)
            .with_response_file("projects/get-projects.json"),
    );
    register_test("projects/projects-list.trycmd");
}

#[test]
fn command_projects_list_help() {
    register_test("projects/projects-list-help.trycmd");
}
