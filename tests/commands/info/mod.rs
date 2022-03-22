use mockito::server_url;
use trycmd::TestCases;

use crate::common::{create_testcase, mock_endpoint, EndpointOptions};

#[test]
fn command_info_no_token() {
    let t = TestCases::new();
    t.case("tests/cmd/info/info-no-token.trycmd");
}

#[test]
fn command_info_basic() {
    let _server = mock_endpoint(
        EndpointOptions::new("GET", "/api/0/", 200)
            .with_response_file("tests/responses/info/get-info.json"),
    );
    let t = create_testcase();
    t.case("tests/cmd/info/info-basic.trycmd");
    t.extend_vars([("[SERVER]", server_url())]).unwrap();
}
