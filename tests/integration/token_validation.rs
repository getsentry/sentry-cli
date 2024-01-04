use crate::integration::register_test;

#[test]
fn test_warn_invalid_auth_token() {
    register_test("token_validation/warn-invalid-auth-token.trycmd");
}

#[test]
fn test_valid_auth_token() {
    register_test("token_validation/valid-auth-token.trycmd");
}
