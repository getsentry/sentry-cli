use crate::integration::register_test;

#[test]
fn command_help() {
    register_test("help/help.trycmd");
}
