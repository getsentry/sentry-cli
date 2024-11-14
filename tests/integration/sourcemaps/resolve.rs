use crate::integration::TestManager;

#[test]
fn command_sourcemaps_resolve_help() {
    TestManager::new().register_trycmd_test("sourcemaps/sourcemaps-resolve-help.trycmd");
}

#[test]
fn command_sourcemaps_resolve() {
    TestManager::new().register_trycmd_test("sourcemaps/sourcemaps-resolve.trycmd");
}
