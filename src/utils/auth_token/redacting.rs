use crate::utils::auth_token::{AuthToken, ORG_AUTH_TOKEN_PREFIX, USER_TOKEN_PREFIX};
use lazy_static::lazy_static;
use regex::Regex;
use std::borrow::Cow;

pub fn redact_token_from_string<'r>(to_redact: &'r str, replacement: &'r str) -> Cow<'r, str> {
    if AuthToken::from(to_redact).format_recognized() {
        // The string is itself an auth token, redact the whole thing
        Cow::Borrowed(replacement)
    } else {
        // Redact any substrings consisting of non-whitespace characters starting with the org or
        // user auth token prefixes, as these are likely to be auth tokens. Note that this will
        // miss old-style user auth tokens that do not contain the prefix.
        lazy_static! {
            static ref AUTH_TOKEN_REGEX: Regex = Regex::new(&format!(
                "(({ORG_AUTH_TOKEN_PREFIX})|({USER_TOKEN_PREFIX}))\\S+"
            ))
            .unwrap();
        }

        AUTH_TOKEN_REGEX.replace_all(to_redact, replacement)
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::auth_token::redacting::redact_token_from_string;

    #[test]
    fn test_no_redaction() {
        let input = "This string should remain unchanged.";

        let output = redact_token_from_string(input, "[REDACTED]");
        assert_eq!(input, output);
    }

    #[test]
    fn test_redaction() {
        let input = "Here we have a usersntryu_user/auth@#tok3n\\which_should.be3redacted and a sntrys_org_auth_token,too.";
        let expected_output = "Here we have a user[REDACTED] and a [REDACTED]";

        let output = redact_token_from_string(input, "[REDACTED]");
        assert_eq!(expected_output, output);
    }

    #[test]
    fn test_redaction_org_auth_token() {
        let input = "sntrys_\
            eyJpYXQiOjE3MDQyMDU4MDIuMTk5NzQzLCJ1cmwiOiJodHRwOi8vbG9jYWxob3N0OjgwMDAiLCJyZ\
            Wdpb25fdXJsIjoiaHR0cDovL2xvY2FsaG9zdDo4MDAwIiwib3JnIjoic2VudHJ5In0=_\
            lQ5ETt61cHhvJa35fxvxARsDXeVrd0pu4/smF4sRieA";
        let expected_output = "[REDACTED]";

        let output = redact_token_from_string(input, "[REDACTED]");
        assert_eq!(expected_output, output);
    }

    #[test]
    fn test_redaction_old_user_token() {
        let input = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let expected_output = "[REDACTED]";

        let output = redact_token_from_string(input, "[REDACTED]");
        assert_eq!(expected_output, output);
    }
}
