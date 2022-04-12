use crate::integration::register_test;

mod check;
mod upload;

#[test]
fn command_debug_files_help() {
    register_test("debug_files/debug_files-help.trycmd");
}

#[test]
fn command_debug_files_no_subcommand() {
    register_test("debug_files/debug_files-no-subcommand.trycmd");
}
