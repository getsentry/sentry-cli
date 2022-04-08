use crate::integration::register_test;

#[test]
fn command_help() {
    #[cfg(not(windows))]
    register_test("help/help.trycmd");
    #[cfg(windows)]
    register_test("help/help-windows.trycmd");
}
