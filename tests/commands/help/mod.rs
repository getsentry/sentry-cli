use trycmd::TestCases;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[test]
fn command_help() {
    let t = TestCases::new();
    t.case("tests/cmd/help/help.trycmd");
    t.extend_vars([("[VERSION]", VERSION)]).unwrap();
}
