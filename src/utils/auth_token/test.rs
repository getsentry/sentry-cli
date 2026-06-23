//! Unit tests for the auth token module's public interface.

use super::AuthToken;
use rstest::rstest;
use secrecy::ExposeSecret as _;
// Org auth token tests -----------------------------------------------------

#[test]
fn test_valid_org_auth_token() {
    let good_token = String::from(
        "sntrys_\
            eyJpYXQiOjE3MDQyMDU4MDIuMTk5NzQzLCJ1cmwiOiJodHRwOi8vbG9jYWxob3N0OjgwMDAiLCJyZ\
            Wdpb25fdXJsIjoiaHR0cDovL2xvY2FsaG9zdDo4MDAwIiwib3JnIjoic2VudHJ5In0=_\
            lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRieA",
    );

    let token = AuthToken::from(good_token.clone());

    assert!(token.payload().is_some());

    let payload = token.payload().unwrap();
    assert_eq!(payload.org, "sentry");
    assert_eq!(payload.url, "http://localhost:8000");
    assert_eq!(payload.region_url, "http://localhost:8000");
    assert_eq!(payload.base_url(), "http://localhost:8000");

    assert_eq!(good_token, token.raw().expose_secret().clone());

    assert!(token.format_recognized());
}

#[test]
fn test_valid_org_auth_token_region_url_differs() {
    // Payload: {"url":"http://control.example","region_url":"http://region.example","org":"sentry"}
    let good_token = String::from(
        "sntrys_\
            eyJpYXQiOiAxNzA0MjA1ODAyLjE5OTc0MywgInVybCI6ICJodHRwOi8vY29udHJvbC5leGFtcGxlIiw\
            gInJlZ2lvbl91cmwiOiAiaHR0cDovL3JlZ2lvbi5leGFtcGxlIiwgIm9yZyI6ICJzZW50cnkifQ==_\
            lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRieA",
    );

    let token = AuthToken::from(good_token);
    let payload = token.payload().unwrap();

    assert_eq!(payload.org, "sentry");
    assert_eq!(payload.url, "http://control.example");
    assert_eq!(payload.region_url, "http://region.example");
    // base_url prefers region_url when present.
    assert_eq!(payload.base_url(), "http://region.example");
}

#[test]
fn test_valid_org_auth_token_missing_region_url() {
    // Legacy token with no region_url field at all; base_url falls back to url.
    // Payload: {"url":"http://legacy.example","org":"sentry"}
    let good_token = String::from(
        "sntrys_\
            eyJpYXQiOiAxNzA0MjA1ODAyLjE5OTc0MywgInVybCI6ICJodHRwOi8vbGVnYWN5LmV4YW1wbGUiL\
            CAib3JnIjogInNlbnRyeSJ9_\
            lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRieA",
    );

    let token = AuthToken::from(good_token);
    let payload = token.payload().unwrap();

    assert_eq!(payload.org, "sentry");
    assert_eq!(payload.url, "http://legacy.example");
    assert!(payload.region_url.is_empty());
    assert_eq!(payload.base_url(), "http://legacy.example");
}

#[test]
fn test_valid_org_auth_token_missing_url() {
    let good_token = String::from(
        "sntrys_\
            eyJpYXQiOjE3MDQyMDU4MDIuMTk5NzQzLCJ1cmwiOm51bGwsInJlZ2lvbl91cmwiOiJodHRwOi8vb\
            G9jYWxob3N0OjgwMDAiLCJvcmciOiJzZW50cnkifQ==_\
            lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRieA",
    );

    let token = AuthToken::from(good_token.clone());

    assert!(token.payload().is_some());

    let payload = token.payload().unwrap();
    assert_eq!(payload.org, "sentry");
    assert!(payload.url.is_empty());
    // url is null but region_url is present, so base_url falls back to region_url.
    assert_eq!(payload.region_url, "http://localhost:8000");
    assert_eq!(payload.base_url(), "http://localhost:8000");

    assert_eq!(good_token, token.raw().expose_secret().clone());

    assert!(token.format_recognized());
}

// User auth token tests ----------------------------------------------------

#[rstest]
#[case::no_prefix("c66aee1348a6e7a0993145d71cf8fa529ed09ee13dd5177b5f692e9f6ca38c30")]
#[case::with_prefix("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")]
fn test_valid_user_auth_token(#[case] token_str: &'static str) {
    let good_token = String::from(token_str);

    let token = AuthToken::from(good_token.clone());

    assert!(token.payload().is_none());
    assert_eq!(good_token, token.raw().expose_secret().clone());

    assert!(token.format_recognized());
}

// Unknown auth token tests -------------------------------------------------

#[rstest]
// Cases similar to org auth token -----------------------------------------
#[case::wrong_prefix(
    "sentry_\
    eyJpYXQiOjE3MDQyMDU4MDIuMTk5NzQzLCJ1cmwiOiJodHRwOi8vbG9jYWxob3N0OjgwMDAiLCJyZ\
    Wdpb25fdXJsIjoiaHR0cDovL2xvY2FsaG9zdDo4MDAwIiwib3JnIjoic2VudHJ5In0=_\
    lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRieA"
)]
#[case::one_underscore(
    "sntrys_\
    eyJpYXQiOjE3MDQyMDU4MDIuMTk5NzQzLCJ1cmwiOiJodHRwOi8vbG9jYWxob3N0OjgwMDAiLCJyZ\
    Wdpb25fdXJsIjoiaHR0cDovL2xvY2FsaG9zdDo4MDAwIiwib3JnIjoic2VudHJ5In0="
)]
#[case::three_underscores(
    "sntrys_\
    eyJpYXQiOjE3MDQyMDU4MDIuMTk5NzQzLCJ1cmwiOiJodHRwOi8vbG9jYWxob3N0OjgwMDAiLCJyZ\
    Wdpb25fdXJsIjoiaHR0cDovL2xvY2FsaG9zdDo4MDAwIiwib3JnIjoic2VudHJ5In0=_\
    lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRieA_"
)]
#[case::payload_invalid_base64(
    "sntrys_\
    eyJpYXQiOjE3MDQyMDU4MDIuMT5NzQzLCJ1cmwiOiJodHRwOi8vbG9jYWxob3N0OjgwMDAiLCJyZ\
    Wdpb25fdXJsIjoiaHR0cDovL2xvY2FsaG9zdDo4MDAwIiwib3JnIjoic2VudHJ5In0=_\
    lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRieA"
)]
#[case::valid_base64_invalid_json(
    "sntrys_\
    eyJpYXQiOiAxNzA0MjA1ODAyLjE5OTc0MywgInVybCI6ICJodHRwOi8vbG9jYWxob3N0OjgwMDAiL\
    CAicmVnaW9uX3VybCI6ICJodHRwOi8vbG9jYWxob3N0OjgwMDAiLCAib3JqIjogInNlbnRyeSJ9_\
    lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRieA"
)]
#[case::missing_payload("sntrys__lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRieA")]
#[case::missing_secret(
    "sntrys_\
    eyJpYXQiOjE3MDQyMDU4MDIuMTk5NzQzLCJ1cmwiOiJodHRwOi8vbG9jYWxob3N0OjgwMDAiLCJyZ\
    Wdpb25fdXJsIjoiaHR0cDovL2xvY2FsaG9zdDo4MDAwIiwib3JnIjoic2VudHJ5In0=_"
)]
#[case::secret_missing_last_char(
    "sntrys_\
    eyJpYXQiOjE3MDQyMDU4MDIuMTk5NzQzLCJ1cmwiOiJodHRwOi8vbG9jYWxob3N0OjgwMDAiLCJyZ\
    Wdpb25fdXJsIjoiaHR0cDovL2xvY2FsaG9zdDo4MDAwIiwib3JnIjoic2VudHJ5In0=_\
    lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRie"
)]
#[case::secret_extra_char(
    "sntrys_\
    eyJpYXQiOjE3MDQyMDU4MDIuMTk5NzQzLCJ1cmwiOiJodHRwOi8vbG9jYWxob3N0OjgwMDAiLCJyZ\
    Wdpb25fdXJsIjoiaHR0cDovL2xvY2FsaG9zdDo4MDAwIiwib3JnIjoic2VudHJ5In0=_\
    lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRieAx"
)]
// Cases similar to user auth token ----------------------------------------
#[case::thirty_one_bytes("c66aee1348a6e7a0993145d71cf8fa529ed09ee13dd5177b5f692e9f6ca38c")]
#[case::thirty_three_bytes("c66aee1348a6e7a0993145d71cf8fa529ed09ee13dd5177b5f692e9f6ca38c3000")]
#[case::invalid_hex("c66aee1348a6g7a0993145d71cf8fa529ed09ee13dd5177b5f692e9f6ca38c30")]
#[case::sixty_three_characters("c66aee1348a6e7a0993145d71cf8fa529ed09ee13dd5177b5f692e9f6ca38c3")]
#[case::sixty_five_characters("c66aee1348a6e7a0993145d71cf8fa529ed09ee13dd5177b5f692e9f6ca38c300")]
#[case::prefix_only("sntryu_")]
#[case::prefix_sixty_three_characters(
    "sntryu_c66aee1348a6e7a0993145d71cf8fa529ed09ee13dd5177b5f692e9f6ca38c3"
)]
#[case::wrong_prefix("sntryt_c66aee1348a6e7a0993145d71cf8fa529ed09ee13dd5177b5f692e9f6ca38c30")]
fn test_unknown_auth_token(#[case] token_str: &'static str) {
    let token = AuthToken::from(token_str.to_owned());

    assert_eq!(token_str, token.raw().expose_secret().clone());
    assert!(token.payload().is_none());

    assert!(!token.format_recognized());
}
