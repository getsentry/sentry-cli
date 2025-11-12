use crate::integration::TestManager;

mod inject;
mod resolve;
mod upload;

#[test]
fn command_sourcemaps_help() {
    TestManager::new().register_trycmd_test("sourcemaps/sourcemaps-help.trycmd");
}

#[test]
fn command_sourcemaps_no_subcommand() {
    TestManager::new().register_trycmd_test("sourcemaps/sourcemaps-no-subcommand.trycmd");
}
