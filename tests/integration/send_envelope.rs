use crate::integration::register_test;

#[test]
fn command_send_envelope_help() {
    register_test("send_envelope/send_envelope-help.trycmd");
}

#[test]
fn command_send_envelope_no_file() {
    register_test("send_envelope/send_envelope-no-file.trycmd");
}

#[test]
fn command_send_envelope_file() {
    register_test("send_envelope/send_envelope-file.trycmd");
}

#[test]
fn command_send_envelope_with_logging() {
    register_test("send_envelope/send_envelope-file-log.trycmd");
}
