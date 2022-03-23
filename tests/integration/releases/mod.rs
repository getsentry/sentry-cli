use crate::integration::register_test;

mod delete;
mod finalize;
mod info;
mod list;
mod new;

#[test]
fn command_releases_no_subcommand() {
    register_test("releases/releases-no-subcommand.trycmd");
}
