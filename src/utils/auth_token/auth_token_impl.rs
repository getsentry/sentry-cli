//! Defines the AuthToken type, which stores a Sentry auth token.

use super::AuthTokenPayload;
use super::{OrgAuthToken, UserAuthToken};
use secrecy::SecretString;

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
    pub fn raw(&self) -> &SecretString {
        self.0.raw()
    }

    /// Returns whether the auth token follows a recognized format. If this function returns false,
    /// that indicates that the auth token might not be valid, since it failed our soft validation.
    pub fn format_recognized(&self) -> bool {
        !matches!(self.0, AuthTokenInner::Unknown(_))
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

/// Inner representation of AuthToken type, containing all possible auth token types.
#[derive(Debug, Clone)]
enum AuthTokenInner {
    /// Represents an org auth token.
    Org(OrgAuthToken),

    /// Represents a user auth token.
    User(UserAuthToken),

    /// Represents an auth token that has an unrecognized format.
    Unknown(SecretString),
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
            AuthTokenInner::Unknown(auth_string.into())
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
    fn raw(&self) -> &SecretString {
        match self {
            AuthTokenInner::Org(ref org_auth_token) => org_auth_token.raw(),
            AuthTokenInner::User(user_auth_token) => user_auth_token.raw(),
            AuthTokenInner::Unknown(auth_string) => auth_string,
        }
    }
}
