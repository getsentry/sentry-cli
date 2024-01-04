use super::{AuthTokenParseError, Result};

const USER_TOKEN_BYTES: usize = 32;

/// Represents a valid User Auth Token.
#[derive(Debug, Clone)]
pub struct UserAuthToken(String);

impl UserAuthToken {
    /// Constructs a new UserAuthToken from a string. Returns an error if the string is not a valid user auth token.
    fn construct_from_string(auth_string: String) -> Result<Self> {
        let bytes = data_encoding::HEXLOWER_PERMISSIVE.decode(auth_string.as_bytes());

        if bytes.is_ok() && bytes.unwrap().len() == USER_TOKEN_BYTES {
            Ok(UserAuthToken(auth_string))
        } else {
            Err(AuthTokenParseError)
        }
    }

    /// Retrieves a reference to the auth token string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for UserAuthToken {
    type Error = AuthTokenParseError;

    /// Constructs a new UserAuthToken from a string. Returns an error if the string is not a valid user auth token.
    fn try_from(value: String) -> Result<UserAuthToken> {
        UserAuthToken::construct_from_string(value)
    }
}
