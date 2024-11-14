use crate::integration::TestManager;

#[test]
fn command_update() {
    TestManager::new().register_trycmd_test("update/*.trycmd");
}
