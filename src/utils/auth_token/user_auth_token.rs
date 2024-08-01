use super::{AuthTokenParseError, Result, USER_TOKEN_PREFIX};
use secrecy::SecretString;

const USER_TOKEN_BYTES: usize = 32;

/// Represents a valid User Auth Token.
#[derive(Debug, Clone)]
pub struct UserAuthToken(SecretString);

impl UserAuthToken {
    /// Constructs a new UserAuthToken from a string. Returns an error if the string is not a valid user auth token.
    fn construct_from_string(auth_string: String) -> Result<Self> {
        let secret_portion = auth_string
            .strip_prefix(USER_TOKEN_PREFIX)
            .unwrap_or(&auth_string);

        let bytes = data_encoding::HEXLOWER_PERMISSIVE.decode(secret_portion.as_bytes());

        if bytes.is_ok() && bytes.unwrap().len() == USER_TOKEN_BYTES {
            Ok(UserAuthToken(auth_string.into()))
        } else {
            Err(AuthTokenParseError)
        }
    }

    /// Retrieves a reference to the auth token string.
    pub fn raw(&self) -> &SecretString {
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
