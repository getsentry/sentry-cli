use crate::integration::{mock_endpoint, register_test, EndpointOptions};

#[test]
fn command_deploys_list() {
    let _server = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/organizations/wat-org/releases/wat-release/deploys/",
            200,
        )
        .with_response_file("deploys/get-deploys.json"),
    );
    register_test("deploys/deploys-list.trycmd");
}
