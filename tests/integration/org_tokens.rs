use crate::integration::register_test;

#[test]
fn org_token_url_mismatch() {
    register_test("org_tokens/url-mismatch.trycmd");
}

#[test]
fn org_token_org_mismatch() {
    register_test("org_tokens/org-mismatch.trycmd");
}

#[test]
fn org_token_url_match() {
    register_test("org_tokens/url-match.trycmd");
}

#[test]
fn org_token_region_url_match() {
    register_test("org_tokens/url-region-match.trycmd");
}

#[test]
fn org_token_org_match() {
    register_test("org_tokens/org-match.trycmd");
}

#[test]
fn org_token_url_works() {
    register_test("org_tokens/url-works.trycmd");
}

#[test]
fn org_token_url_mismatch_empty_token() {
    register_test("org_tokens/url-mismatch-empty-token.trycmd");
}
