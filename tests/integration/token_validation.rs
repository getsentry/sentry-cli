use crate::integration::register_test;

#[test]
fn test_warn_invalid_auth_token() {
    register_test("token_validation/*.trycmd");
}
