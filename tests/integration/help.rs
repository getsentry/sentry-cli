use crate::integration::register_test;

#[test]
fn command_help() {
    #[cfg(not(windows))]
    let _t = register_test("help/help.trycmd");
    #[cfg(windows)]
    let _t = register_test("help/help-windows.trycmd");
}
