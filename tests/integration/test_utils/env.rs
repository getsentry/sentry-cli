//! Utilities for setting environment variables in integration tests.

use std::borrow::Cow;

/// Set the environment variables, which should be set for all integration tests,
/// using the provided setter function.
/// The setter function takes as parameters the environment variable name, and the
/// value to set it to, in that order.
pub fn set(mut setter: impl FnMut(&'static str, Cow<'static, str>)) {
    let dsn = format!("http://test@{}/1337", mockito::server_address()).into();

    setter("SENTRY_INTEGRATION_TEST", "1".into());
    setter("SENTRY_ORG", "wat-org".into());
    setter("SENTRY_PROJECT", "wat-project".into());
    setter("SENTRY_URL", mockito::server_url().into());
    setter("SENTRY_DSN", dsn);
}

/// Set the auth token environment variable using the provided setter function.
pub fn set_auth_token(setter: impl FnOnce(&'static str, Cow<'static, str>)) {
    setter(
        "SENTRY_AUTH_TOKEN",
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
    );
}
