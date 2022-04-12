use crate::integration::register_test;

#[test]
fn command_update_help() {
    register_test("update/update-help.trycmd");
}

#[test]
fn command_update() {
    register_test("update/update.trycmd");
}
