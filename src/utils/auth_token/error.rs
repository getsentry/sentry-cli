use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
    result::Result as StdResult,
};

/// Represents an error that occurs when parsing an auth token.
#[derive(Debug, PartialEq)]
pub struct AuthTokenParseError;

/// Convenience type alias for auth token parsing results.
pub type Result<T> = StdResult<T, AuthTokenParseError>;

impl Display for AuthTokenParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Invalid Sentry auth token!")?;
        Ok(())
    }
}

impl Error for AuthTokenParseError {}
