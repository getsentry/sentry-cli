use crate::integration::TestManager;

#[test]
fn command_login() {
    TestManager::new().register_trycmd_test("login/*.trycmd");
}
