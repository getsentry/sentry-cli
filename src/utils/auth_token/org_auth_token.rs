use super::{AuthTokenParseError, Result, ORG_AUTH_TOKEN_PREFIX};
use secrecy::SecretString;
use serde::{Deserialize, Deserializer};

const ORG_TOKEN_SECRET_BYTES: usize = 32;

/// Represents a valid org auth token.
#[derive(Debug, Clone)]
pub struct OrgAuthToken {
    auth_string: SecretString,
    pub payload: AuthTokenPayload,
}

/// Represents the payload data of an org auth token.
#[derive(Clone, Debug, Deserialize)]
pub struct AuthTokenPayload {
    pub region_url: String,
    pub org: String,

    // URL may be missing from some old auth tokens, see getsentry/sentry#57123
    #[serde(deserialize_with = "url_deserializer")]
    pub url: String,
}

/// Deserializes a URL from a string, returning an empty string if the URL is missing or null.
fn url_deserializer<'de, D>(deserializer: D) -> std::result::Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Option::deserialize(deserializer).map(|url| url.unwrap_or_default())
}

impl OrgAuthToken {
    /// Parses the payload of an org auth token from the encoded payload string extracted from the token
    /// string. Returns an error if the payload cannot be decoded or if the deceded payload is not valid JSON.
    fn parse_payload(payload_encoded: &str) -> Result<AuthTokenPayload> {
        let payload_bytes = data_encoding::BASE64
            .decode(payload_encoded.as_bytes())
            .map_err(|_| AuthTokenParseError)?;

        let payload = String::from_utf8(payload_bytes).map_err(|_| AuthTokenParseError)?;

        serde_json::from_str(&payload).map_err(|_| AuthTokenParseError)
    }

    /// Validates the secret segment of an org auth token. Returns an error if the secret is not valid.
    fn validate_secret(secret: &str) -> Result<()> {
        let num_bytes = data_encoding::BASE64_NOPAD
            .decode(secret.as_bytes())
            .map(|bytes| bytes.len());

        match num_bytes {
            Ok(ORG_TOKEN_SECRET_BYTES) => Ok(()),
            _ => Err(AuthTokenParseError),
        }
    }

    /// Constructs a new OrgAuthToken from a string. Returns an error if the string is not a valid org auth token.
    fn construct_from_string(auth_string: String) -> Result<OrgAuthToken> {
        if !auth_string.starts_with(ORG_AUTH_TOKEN_PREFIX) {
            return Err(AuthTokenParseError);
        }

        let mut segment_split = auth_string.split('_');
        segment_split.next(); // Skip the prefix; we already validated it.

        let payload_encoded = segment_split.next().ok_or(AuthTokenParseError)?;
        let payload = OrgAuthToken::parse_payload(payload_encoded)?;

        let secret = segment_split.next().ok_or(AuthTokenParseError)?;
        OrgAuthToken::validate_secret(secret)?;

        if segment_split.next().is_some() {
            return Err(AuthTokenParseError);
        }

        let auth_string = auth_string.into();

        Ok(OrgAuthToken {
            auth_string,
            payload,
        })
    }

    /// Retrieves a reference to the auth token string.
    pub fn raw(&self) -> &SecretString {
        &self.auth_string
    }
}

impl TryFrom<String> for OrgAuthToken {
    type Error = AuthTokenParseError;

    /// Constructs a new OrgAuthToken from a string. Returns an error if the string is not a valid org auth token.
    fn try_from(value: String) -> Result<OrgAuthToken> {
        OrgAuthToken::construct_from_string(value)
    }
}
