use crate::integration::TestManager;

#[test]
fn command_proguard_uuid() {
    TestManager::new().register_trycmd_test("proguard/proguard-uuid*.trycmd");
}
