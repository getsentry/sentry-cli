use crate::integration::register_test;

#[test]
fn command_uninstall_help() {
    register_test("uninstall/uninstall-help.trycmd");
}

#[test]
fn command_uninstall() {
    #[cfg(not(windows))]
    register_test("uninstall/uninstall.trycmd");
    #[cfg(windows)]
    register_test("uninstall/uninstall-windows.trycmd");
}
