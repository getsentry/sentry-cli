use crate::integration::TestManager;

mod delete;
mod finalize;
mod info;
mod list;
mod new;

#[test]
fn command_releases_help() {
    TestManager::new().register_trycmd_test("releases/releases-help.trycmd");
}

#[test]
fn command_releases_no_subcommand() {
    TestManager::new().register_trycmd_test("releases/releases-no-subcommand.trycmd");
}
