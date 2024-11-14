use crate::integration::TestManager;

#[test]
fn command_bash_hook() {
    TestManager::new().register_trycmd_test("bash_hook/*.trycmd");
}
