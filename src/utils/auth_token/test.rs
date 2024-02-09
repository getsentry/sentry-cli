//! Unit tests for the auth token module's public interface.

use super::AuthToken;
use rstest::rstest;
use testing_logger::CapturedLog;

/// Asserts that the logs vector is empty.
#[allow(clippy::ptr_arg)] // This function signature is required by testing_logger
fn assert_no_logs(logs: &Vec<CapturedLog>) {
    assert!(logs.is_empty());
}

/// Asserts that the logs vector contains exactly one warning.
#[allow(clippy::ptr_arg)] // This function signature is required by testing_logger
fn assert_one_warning(logs: &Vec<CapturedLog>) {
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].level, log::Level::Warn);
}

// Org auth token tests -----------------------------------------------------

#[test]
fn test_valid_org_auth_token() {
    let good_token = String::from(
        "sntrys_\
            eyJpYXQiOjE3MDQyMDU4MDIuMTk5NzQzLCJ1cmwiOiJodHRwOi8vbG9jYWxob3N0OjgwMDAiLCJyZ\
            Wdpb25fdXJsIjoiaHR0cDovL2xvY2FsaG9zdDo4MDAwIiwib3JnIjoic2VudHJ5In0=_\
            lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRieA",
    );

    testing_logger::setup();
    let token = AuthToken::from(good_token.clone());

    assert!(token.payload().is_some());

    let payload = token.payload().unwrap();
    assert_eq!(payload.org, "sentry");
    assert_eq!(payload.url, "http://localhost:8000");

    assert_eq!(good_token, token.to_string());

    testing_logger::validate(assert_no_logs);
}

#[test]
fn test_valid_org_auth_token_missing_url() {
    let good_token = String::from(
        "sntrys_\
            eyJpYXQiOjE3MDQyMDU4MDIuMTk5NzQzLCJ1cmwiOm51bGwsInJlZ2lvbl91cmwiOiJodHRwOi8vb\
            G9jYWxob3N0OjgwMDAiLCJvcmciOiJzZW50cnkifQ==_\
            lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRieA",
    );

    testing_logger::setup();
    let token = AuthToken::from(good_token.clone());

    assert!(token.payload().is_some());

    let payload = token.payload().unwrap();
    assert_eq!(payload.org, "sentry");
    assert!(payload.url.is_empty());

    assert_eq!(good_token, token.to_string());

    testing_logger::validate(assert_no_logs);
}

// User auth token tests ----------------------------------------------------

#[test]
fn test_valid_user_auth_token() {
    let good_token =
        String::from("c66aee1348a6e7a0993145d71cf8fa529ed09ee13dd5177b5f692e9f6ca38c30");

    testing_logger::setup();
    let token = AuthToken::from(good_token.clone());

    assert!(token.payload().is_none());
    assert_eq!(good_token, token.to_string());

    testing_logger::validate(assert_no_logs);
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
fn test_unknown_auth_token(#[case] token_str: &'static str) {
    testing_logger::setup();
    let token = AuthToken::from(token_str.to_owned());

    assert_eq!(token_str, token.to_string());
    assert!(token.payload().is_none());

    testing_logger::validate(assert_one_warning);
}
