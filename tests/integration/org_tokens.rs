use crate::integration::TestManager;

#[test]
fn org_token() {
    TestManager::new().register_trycmd_test("org_tokens/*.trycmd");
}
