use crate::integration;

use super::EndpointOptions;

#[test]
fn command_send_envelope() {
    let _server =
        integration::mock_endpoint(EndpointOptions::new("POST", "/api/1337/envelope/", 200));
    integration::register_test("send_envelope/*.trycmd");
}
