use crate::integration::register_test;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[test]
fn command_help() {
    let t = register_test("help/help.trycmd");
    t.extend_vars([("[VERSION]", VERSION)]).unwrap();
}
