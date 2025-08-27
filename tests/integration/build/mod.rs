use crate::integration::TestManager;

mod upload;

#[test]
fn command_build_help() {
    TestManager::new().register_trycmd_test("build/build-help.trycmd");
}
