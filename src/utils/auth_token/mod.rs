//! This module provides logic for storing, parsing, and validating Sentry auth tokens.

mod auth_token_impl;
mod error;
mod org_auth_token;
mod redacting;
mod user_auth_token;

pub use auth_token_impl::AuthToken;
pub use org_auth_token::AuthTokenPayload;
pub use redacting::redact_token_from_string;

use error::{AuthTokenParseError, Result};
use org_auth_token::OrgAuthToken;
use user_auth_token::UserAuthToken;

#[cfg(test)]
mod test;

const ORG_AUTH_TOKEN_PREFIX: &str = "sntrys_";
const USER_TOKEN_PREFIX: &str = "sntryu_";
