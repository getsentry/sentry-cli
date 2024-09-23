use crate::integration;

use super::EndpointOptions;

#[test]
fn command_send_envelope_help() {
    integration::register_test("send_envelope/send_envelope-help.trycmd");
}

#[test]
fn command_send_envelope_no_file() {
    integration::register_test("send_envelope/send_envelope-no-file.trycmd");
}

#[test]
fn command_send_envelope_file() {
    let _server =
        integration::mock_endpoint(EndpointOptions::new("POST", "/api/1337/envelope/", 200));
    integration::register_test("send_envelope/send_envelope-file.trycmd");
}

#[test]
fn command_send_envelope_with_logging() {
    let _server =
        integration::mock_endpoint(EndpointOptions::new("POST", "/api/1337/envelope/", 200));
    integration::register_test("send_envelope/send_envelope-file-log.trycmd");
}
