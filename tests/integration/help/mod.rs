use crate::integration::register_test;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[test]
fn command_help() {
    #[cfg(not(windows))]
    let t = register_test("help/help.trycmd");
    #[cfg(windows)]
    let t = register_test("help/help-windows.trycmd");
    t.insert_var("[VERSION]", VERSION).unwrap();
}
