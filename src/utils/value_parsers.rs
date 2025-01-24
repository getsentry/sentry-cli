use crate::utils::auth_token::AuthToken;
use anyhow::{anyhow, Result};
use std::convert::Infallible;

/// Parse key:value pair from string, used as a value_parser for Clap arguments
pub fn kv_parser(s: &str) -> Result<(String, String)> {
    s.split_once(':')
        .map(|(k, v)| (k.into(), v.into()))
        .ok_or_else(|| anyhow!("`{s}` is missing a `:`"))
}

/// Parse an AuthToken, and warn if the format is unrecognized
// Clap requires parsers to return a Result, hence why this function returns
// a Result, violating the clippy::unnecessary_wraps lint.
#[expect(clippy::unnecessary_wraps)]
pub fn auth_token_parser(s: &str) -> Result<AuthToken, Infallible> {
    let token = AuthToken::from(s);
    if !token.format_recognized() {
        log::warn!("Unrecognized auth token format. Ensure you copied your token correctly.");
    }

    Ok(token)
}
