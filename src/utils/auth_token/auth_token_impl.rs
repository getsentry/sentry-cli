//! Defines the AuthToken type, which stores a Sentry auth token.

use super::AuthTokenPayload;
use super::{OrgAuthToken, UserAuthToken};
use std::fmt::{Display, Formatter, Result};

/// Represents a (soft) validated Sentry auth token.
#[derive(Debug, Clone)]
pub struct AuthToken(AuthTokenInner);

impl AuthToken {
    /// Constructs a new AuthToken from a string. Logs a warning if the auth token's
    /// format is unrecognized.
    fn new(auth_string: String) -> Self {
        AuthToken(AuthTokenInner::new(auth_string))
    }

    /// Returns the payload of the auth token, if it is an org auth token.
    pub fn payload(&self) -> Option<&AuthTokenPayload> {
        self.0.payload()
    }

    /// Retrieves a reference to the auth token string.
    fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for AuthToken {
    /// Constructs a new AuthToken from a string. Logs a warning if the auth token's
    /// format is unrecognized.
    fn from(auth_string: String) -> Self {
        AuthToken::new(auth_string)
    }
}

impl From<&str> for AuthToken {
    /// Constructs a new AuthToken from a string. Logs a warning if the auth token's
    /// format is unrecognized.
    fn from(auth_string: &str) -> Self {
        AuthToken::from(auth_string.to_owned())
    }
}

impl Display for AuthToken {
    /// Displays the auth token string.
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.as_str())?;
        Ok(())
    }
}

/// Inner representation of AuthToken type, containing all possible auth token types.
#[derive(Debug, Clone)]
enum AuthTokenInner {
    /// Represents an org auth token.
    Org(OrgAuthToken),

    /// Represents a user auth token.
    User(UserAuthToken),

    /// Represents an auth token that has an unrecognized format.
    Unknown(String),
}

impl AuthTokenInner {
    /// Constructs a new AuthTokenInner from a string. Logs a warning if the auth token's
    /// format is unrecognized; i.e. if an Unknown enum variant is returned.
    fn new(auth_string: String) -> Self {
        if let Ok(org_auth_token) = OrgAuthToken::try_from(auth_string.clone()) {
            AuthTokenInner::Org(org_auth_token)
        } else if let Ok(user_auth_token) = UserAuthToken::try_from(auth_string.clone()) {
            AuthTokenInner::User(user_auth_token)
        } else {
            log::warn!(
                "Unrecognized auth token format!\n\tHint: Did you copy your token correctly?"
            );
            AuthTokenInner::Unknown(auth_string)
        }
    }

    /// Returns the payload of the auth token, if it is an org auth token. Returns None for
    /// all other auth token types.
    fn payload(&self) -> Option<&AuthTokenPayload> {
        match self {
            AuthTokenInner::Org(org_auth_token) => Some(&org_auth_token.payload),
            _ => None,
        }
    }

    /// Retrieves a reference to the auth token string.
    fn as_str(&self) -> &str {
        match self {
            AuthTokenInner::Org(ref org_auth_token) => org_auth_token.as_str(),
            AuthTokenInner::User(user_auth_token) => user_auth_token.as_str(),
            AuthTokenInner::Unknown(auth_string) => auth_string,
        }
    }
}
