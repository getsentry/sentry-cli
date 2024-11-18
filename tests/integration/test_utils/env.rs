//! Utilities for setting environment variables in integration tests.

use std::borrow::Cow;

use mockito::ServerGuard;

pub struct MockServerInfo {
    url: String,
    host_with_port: String,
}

impl From<&ServerGuard> for MockServerInfo {
    fn from(server: &ServerGuard) -> Self {
        Self {
            url: server.url(),
            host_with_port: server.host_with_port(),
        }
    }
}

/// Set the environment variables, which should be set for all integration tests,
/// using the provided setter function.
/// The setter function takes as parameters the environment variable name, and the
/// value to set it to, in that order.
/// Information about the mock server is needed to set the SENTRY_URL and SENTRY_DSN.
/// This is obtained from `TestManager`.
pub fn set(server_info: MockServerInfo, mut setter: impl FnMut(&str, Cow<str>)) {
    let dsn = format!("http://test@{}/1337", server_info.host_with_port).into();

    setter("SENTRY_INTEGRATION_TEST", "1".into());
    setter("SENTRY_ORG", "wat-org".into());
    setter("SENTRY_PROJECT", "wat-project".into());
    setter("SENTRY_URL", server_info.url.into());
    setter("SENTRY_DSN", dsn);
    setter("RUST_BACKTRACE", "0".into());
}

/// Set the auth token environment variable using the provided setter function.
pub fn set_auth_token(setter: impl FnOnce(&'static str, Cow<'static, str>)) {
    setter(
        "SENTRY_AUTH_TOKEN",
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
    );
}

/// Set all environment variables, including the auth token and the environments
/// set by `set`.
pub fn set_all(server_info: MockServerInfo, mut setter: impl FnMut(&str, Cow<str>)) {
    set(server_info, &mut setter);
    set_auth_token(setter);
}
