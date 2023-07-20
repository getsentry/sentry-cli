use crate::integration::register_test;

mod list;

#[test]
fn command_issues_help() {
    register_test("issues/issues-help.trycmd");
}
