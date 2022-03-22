use crate::common::create_testcase;

mod delete;
mod finalize;
mod info;
mod list;
mod new;

#[test]
fn command_releases_no_subcommand() {
    let t = create_testcase();
    t.case("tests/cmd/releases/releases-no-subcommand.trycmd");
}
