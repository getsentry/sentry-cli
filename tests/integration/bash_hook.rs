use crate::integration::register_test;

#[test]
fn command_bash_hook() {
    register_test("bash_hook/*.trycmd");
}
