use crate::integration::{mock_endpoint, register_test, EndpointOptions};

#[test]
fn command_organizations_list() {
    let _server = mock_endpoint(
        EndpointOptions::new("GET", "/api/0/organizations/?cursor=", 200)
            .with_response_file("organizations/get-organizations.json"),
    );
    register_test("organizations/organizations-list.trycmd");
}

#[test]
fn command_organizations_list_help() {
    register_test("organizations/organizations-list-help.trycmd");
}
