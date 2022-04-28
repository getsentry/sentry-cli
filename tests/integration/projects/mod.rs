use crate::integration::register_test;

mod list;

#[test]
fn command_projects_help() {
    register_test("projects/projects-help.trycmd");
}

#[test]
fn command_projects_no_subcommand() {
    register_test("projects/projects-no-subcommand.trycmd");
}
