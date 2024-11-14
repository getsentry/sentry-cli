use crate::integration::TestManager;

mod list;
mod new;

#[test]
fn command_deploys_help() {
    TestManager::new().register_trycmd_test("deploys/deploys-help.trycmd");
}

#[test]
fn command_deploys_no_subcommand() {
    TestManager::new().register_trycmd_test("deploys/deploys-no-subcommand.trycmd");
}
