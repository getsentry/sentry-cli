use crate::integration::register_test;

mod list;
mod new;

#[test]
fn command_deploys_help() {
    register_test("deploys/deploys-help.trycmd");
}

#[test]
fn command_deploys_no_subcommand() {
    register_test("deploys/deploys-no-subcommand.trycmd");
}
