use crate::integration::TestManager;

#[test]
fn test_invalid_env() {
    TestManager::new().register_trycmd_test("invalid_env/invalid-env.trycmd");
}
