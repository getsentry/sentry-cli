use crate::integration::TestManager;

mod list;

#[test]
fn command_environments_help() {
    TestManager::new().register_trycmd_test("environments/environments-help.trycmd");
}
