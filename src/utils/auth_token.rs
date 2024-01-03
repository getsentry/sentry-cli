use serde::Deserialize;
use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
    result::Result as StdResult,
};

const ORG_AUTH_TOKEN_PREFIX: &'static str = "sntrys_";
const SECRET_BYTES: usize = 32;

/// Represents an auth token that can be used with the Sentry API.
#[derive(Debug, Clone)]
pub struct AuthToken(AuthTokenInner);

/// Represents all differnt types of auth tokens that can be used with the Sentry API.
#[derive(Debug, Clone)]
enum AuthTokenInner {
    Org(OrgAuthToken),
    User(UserAuthToken),

    /// Represents an auth token that has an unrecognized format.
    Unknown(String),
}

/// Represents a valid Org Auth Token.
#[derive(Debug, Clone)]
struct OrgAuthToken {
    auth_string: String,
    payload: AuthTokenPayload,
}

/// Represents a valid User Auth Token.
#[derive(Debug, Clone)]
struct UserAuthToken(String);

#[derive(Debug, PartialEq)]
struct AuthTokenParseError;

type Result<T> = StdResult<T, AuthTokenParseError>;

#[derive(Clone, Debug, Deserialize)]
#[allow(dead_code)] // Otherwise, we get a warning about unused fields
pub struct AuthTokenPayload {
    iat: f64,
    pub url: Option<String>, // URL may be missing from some old auth tokens, see getsentry/sentry#57123
    region_url: String,
    pub org: String,
}

impl AuthToken {
    fn new(auth_string: String) -> Self {
        AuthToken(AuthTokenInner::new(auth_string))
    }

    pub fn payload(&self) -> Option<&AuthTokenPayload> {
        self.0.payload()
    }
}

impl AuthTokenInner {
    fn new(auth_string: String) -> Self {
        if let Ok(org_auth_token) = OrgAuthToken::try_from(auth_string.clone()) {
            AuthTokenInner::Org(org_auth_token)
        } else {
            log::warn!(
                "Unrecognized auth token format!\n\tHint: Did you copy your token correctly?"
            );
            AuthTokenInner::Unknown(auth_string)
        }
    }

    fn payload(&self) -> Option<&AuthTokenPayload> {
        match self {
            AuthTokenInner::Org(org_auth_token) => Some(&org_auth_token.payload),
            _ => None,
        }
    }
}

impl OrgAuthToken {
    fn generate_payload(payload_encoded: &str) -> Result<AuthTokenPayload> {
        let payload_bytes = data_encoding::BASE64
            .decode(payload_encoded.as_bytes())
            .map_err(|_| AuthTokenParseError)?;

        let payload = String::from_utf8(payload_bytes).map_err(|_| AuthTokenParseError)?;

        serde_json::from_str(&payload).map_err(|_| AuthTokenParseError)
    }

    fn validate_secret(secret: &str) -> Result<()> {
        let num_bytes = data_encoding::BASE64_NOPAD
            .decode(secret.as_bytes())
            .map(|bytes| bytes.len());

        match num_bytes {
            Ok(SECRET_BYTES) => Ok(()),
            _ => Err(AuthTokenParseError),
        }
    }

    fn construct_from_string(auth_string: String) -> Result<OrgAuthToken> {
        if !auth_string.starts_with(ORG_AUTH_TOKEN_PREFIX) {
            return Err(AuthTokenParseError);
        }

        let mut segment_split = auth_string.split('_');
        segment_split.next(); // Skip the prefix

        let payload_encoded = segment_split.next().ok_or(AuthTokenParseError)?;
        let payload = OrgAuthToken::generate_payload(payload_encoded)?;

        let secret = segment_split.next().ok_or(AuthTokenParseError)?;
        OrgAuthToken::validate_secret(secret)?;

        if segment_split.next().is_some() {
            return Err(AuthTokenParseError);
        }

        Ok(OrgAuthToken {
            auth_string,
            payload,
        })
    }
}

impl From<String> for AuthToken {
    fn from(auth_string: String) -> Self {
        AuthToken::new(auth_string)
    }
}

impl From<AuthToken> for String {
    fn from(auth_token: AuthToken) -> Self {
        auth_token.0.into()
    }
}

impl From<AuthTokenInner> for String {
    fn from(value: AuthTokenInner) -> Self {
        match value {
            AuthTokenInner::Org(org_auth_token) => org_auth_token.into(),
            AuthTokenInner::User(user_auth_token) => user_auth_token.into(),
            AuthTokenInner::Unknown(auth_string) => auth_string,
        }
    }
}

impl From<UserAuthToken> for String {
    fn from(user_auth_token: UserAuthToken) -> Self {
        user_auth_token.0
    }
}

impl TryFrom<String> for OrgAuthToken {
    type Error = AuthTokenParseError;

    fn try_from(value: String) -> Result<OrgAuthToken> {
        OrgAuthToken::construct_from_string(value)
    }
}

impl From<OrgAuthToken> for String {
    fn from(auth_token: OrgAuthToken) -> Self {
        auth_token.auth_string
    }
}

impl Display for AuthTokenParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Invalid Sentry auth token!")?;
        Ok(())
    }
}

impl Error for AuthTokenParseError {}

#[cfg(test)]
mod test {
    use super::*;
    use testing_logger::CapturedLog;

    fn assert_no_logs(logs: &Vec<CapturedLog>) {
        assert!(logs.is_empty());
    }

    fn assert_one_warning(logs: &Vec<CapturedLog>) {
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].level, log::Level::Warn);
    }

    // Org auth token tests -----------------------------------------------------

    #[test]
    fn test_valid_org_auth_token() {
        let good_token = String::from(
            "sntrys_\
            eyJpYXQiOjE3MDQyMDU4MDIuMTk5NzQzLCJ1cmwiOiJodHRwOi8vbG9jYWxob3N0OjgwMDAiLCJyZ\
            Wdpb25fdXJsIjoiaHR0cDovL2xvY2FsaG9zdDo4MDAwIiwib3JnIjoic2VudHJ5In0=_\
            lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRieA",
        );

        testing_logger::setup();
        let token = AuthToken::from(good_token.clone());

        assert!(token.payload().is_some());

        let payload = token.payload().unwrap();
        assert_eq!(payload.org, "sentry");
        assert_eq!(payload.url, Some(String::from("http://localhost:8000")));

        assert_eq!(good_token, String::from(token));

        testing_logger::validate(assert_no_logs);
    }

    // Unknown auth token tests -------------------------------------------------

    #[test]
    fn test_wrong_prefix() {
        let bad_token = String::from(
            "sentry_\
            eyJpYXQiOjE3MDQyMDU4MDIuMTk5NzQzLCJ1cmwiOiJodHRwOi8vbG9jYWxob3N0OjgwMDAiLCJyZ\
            Wdpb25fdXJsIjoiaHR0cDovL2xvY2FsaG9zdDo4MDAwIiwib3JnIjoic2VudHJ5In0=_\
            lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRieA",
        );

        testing_logger::setup();
        let token = AuthToken::from(bad_token.clone());

        assert!(token.payload().is_none());
        assert_eq!(bad_token, String::from(token));

        testing_logger::validate(assert_one_warning);
    }

    #[test]
    fn test_one_underscore() {
        let bad_token = String::from(
            "sntrys_\
            eyJpYXQiOjE3MDQyMDU4MDIuMTk5NzQzLCJ1cmwiOiJodHRwOi8vbG9jYWxob3N0OjgwMDAiLCJyZ\
            Wdpb25fdXJsIjoiaHR0cDovL2xvY2FsaG9zdDo4MDAwIiwib3JnIjoic2VudHJ5In0=",
        );

        testing_logger::setup();
        let token = AuthToken::from(bad_token.clone());

        assert!(token.payload().is_none());
        assert_eq!(bad_token, String::from(token));

        testing_logger::validate(assert_one_warning);
    }

    #[test]
    fn test_three_underscores() {
        let bad_token = String::from(
            "sntrys_\
            eyJpYXQiOjE3MDQyMDU4MDIuMTk5NzQzLCJ1cmwiOiJodHRwOi8vbG9jYWxob3N0OjgwMDAiLCJyZ\
            Wdpb25fdXJsIjoiaHR0cDovL2xvY2FsaG9zdDo4MDAwIiwib3JnIjoic2VudHJ5In0=_\
            lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRieA_",
        );
        testing_logger::setup();
        let token = AuthToken::from(bad_token.clone());

        assert!(token.payload().is_none());
        assert_eq!(bad_token, String::from(token));

        testing_logger::validate(assert_one_warning);
    }

    #[test]
    fn test_payload_invalid_base64() {
        let bad_token = String::from(
            "sntrys_\
            eyJpYXQiOjE3MDQyMDU4MDIuMT5NzQzLCJ1cmwiOiJodHRwOi8vbG9jYWxob3N0OjgwMDAiLCJyZ\
            Wdpb25fdXJsIjoiaHR0cDovL2xvY2FsaG9zdDo4MDAwIiwib3JnIjoic2VudHJ5In0=_\
            lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRieA",
        );
        testing_logger::setup();
        let token = AuthToken::from(bad_token.clone());

        assert!(token.payload().is_none());
        assert_eq!(bad_token, String::from(token));

        testing_logger::validate(assert_one_warning);
    }

    #[test]
    fn test_payload_valid_base64_invalid_json() {
        let bad_token = String::from(
            "sntrys_\
            eyJpYXQiOiAxNzA0MjA1ODAyLjE5OTc0MywgInVybCI6ICJodHRwOi8vbG9jYWxob3N0OjgwMDAiL\
            CAicmVnaW9uX3VybCI6ICJodHRwOi8vbG9jYWxob3N0OjgwMDAiLCAib3JqIjogInNlbnRyeSJ9_\
            lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRieA",
        );
        testing_logger::setup();
        let token = AuthToken::from(bad_token.clone());

        assert!(token.payload().is_none());
        assert_eq!(bad_token, String::from(token));

        testing_logger::validate(assert_one_warning);
    }

    #[test]
    fn test_missing_payload() {
        let bad_token = String::from("sntrys__lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRieA");
        testing_logger::setup();
        let token = AuthToken::from(bad_token.clone());

        assert!(token.payload().is_none());
        assert_eq!(bad_token, String::from(token));

        testing_logger::validate(assert_one_warning);
    }

    #[test]
    fn test_missing_secret() {
        let bad_token = String::from(
            "sntrys_\
            eyJpYXQiOjE3MDQyMDU4MDIuMTk5NzQzLCJ1cmwiOiJodHRwOi8vbG9jYWxob3N0OjgwMDAiLCJyZ\
            Wdpb25fdXJsIjoiaHR0cDovL2xvY2FsaG9zdDo4MDAwIiwib3JnIjoic2VudHJ5In0=_",
        );
        testing_logger::setup();
        let token = AuthToken::from(bad_token.clone());

        assert!(token.payload().is_none());
        assert_eq!(bad_token, String::from(token));

        testing_logger::validate(assert_one_warning);
    }

    #[test]
    fn test_secret_missing_last_char() {
        let bad_token = String::from(
            "sntrys_\
            eyJpYXQiOjE3MDQyMDU4MDIuMTk5NzQzLCJ1cmwiOiJodHRwOi8vbG9jYWxob3N0OjgwMDAiLCJyZ\
            Wdpb25fdXJsIjoiaHR0cDovL2xvY2FsaG9zdDo4MDAwIiwib3JnIjoic2VudHJ5In0=_\
            lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRie",
        );
        testing_logger::setup();
        let token = AuthToken::from(bad_token.clone());

        assert!(token.payload().is_none());
        assert_eq!(bad_token, String::from(token));

        testing_logger::validate(assert_one_warning);
    }

    #[test]
    fn test_secret_extra_char() {
        let bad_token = String::from(
            "sntrys_\
            eyJpYXQiOjE3MDQyMDU4MDIuMTk5NzQzLCJ1cmwiOiJodHRwOi8vbG9jYWxob3N0OjgwMDAiLCJyZ\
            Wdpb25fdXJsIjoiaHR0cDovL2xvY2FsaG9zdDo4MDAwIiwib3JnIjoic2VudHJ5In0=_\
            lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRieAx",
        );
        testing_logger::setup();
        let token = AuthToken::from(bad_token.clone());

        assert!(token.payload().is_none());
        assert_eq!(bad_token, String::from(token));

        testing_logger::validate(assert_one_warning);
    }
}
