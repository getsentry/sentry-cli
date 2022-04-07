use crate::integration::register_test;

#[test]
fn command_update_help() {
    let _t = register_test("update/update-help.trycmd");
}

#[test]
fn command_update() {
    let _t = register_test("update/update.trycmd");
}
