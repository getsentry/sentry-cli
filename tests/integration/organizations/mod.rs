use crate::integration::register_test;

mod list;

#[test]
fn command_organizations_help() {
    register_test("organizations/organizations-help.trycmd");
}

#[test]
fn command_organizations_no_subcommand() {
    register_test("organizations/organizations-no-subcommand.trycmd");
}
