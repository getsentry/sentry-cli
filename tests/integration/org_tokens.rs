use crate::integration::register_test;

#[test]
fn org_token_url_mismatch() {
    register_test("org_tokens/url-mismatch.trycmd");
}

#[test]
fn org_token_org_mismatch() {
    register_test("org_tokens/org-mismatch.trycmd");
}
