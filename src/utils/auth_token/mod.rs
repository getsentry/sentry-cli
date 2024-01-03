//! This module provides logic for storing, parsing, and validating Sentry auth tokens.

mod auth_token;
mod error;
mod org_auth_token;
mod user_auth_token;

pub use auth_token::AuthToken;
pub use org_auth_token::AuthTokenPayload;

use error::{AuthTokenParseError, Result};
use org_auth_token::OrgAuthToken;
use user_auth_token::UserAuthToken;

#[cfg(test)]
mod test;
