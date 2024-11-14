use crate::integration::TestManager;

#[test]
fn test_warn_invalid_auth_token() {
    TestManager::new().register_trycmd_test("token_validation/*.trycmd");
}
