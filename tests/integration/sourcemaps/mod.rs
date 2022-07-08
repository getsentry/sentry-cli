use crate::integration::register_test;

mod explain;
mod resolve;
mod upload;

#[test]
fn command_sourcemaps_help() {
    register_test("sourcemaps/sourcemaps-help.trycmd");
}

#[test]
fn command_sourcemaps_no_subcommand() {
    register_test("sourcemaps/sourcemaps-no-subcommand.trycmd");
}
