use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use std::fmt::{Display, Formatter, Result};

// Based on https://docs.rs/percent-encoding/1.0.1/src/percent_encoding/lib.rs.html#104
// WHATWG Spec: https://url.spec.whatwg.org/#percent-encoded-bytes
// RFC3986 Reserved Characters: https://www.rfc-editor.org/rfc/rfc3986#section-2.2
const QUERY_ENCODE_SET: AsciiSet = CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'<')
    .add(b'>')
    .add(b'+');

const PATH_SEGMENT_ENCODE_SET: AsciiSet = QUERY_ENCODE_SET
    .add(b'`')
    .add(b'?')
    .add(b'{')
    .add(b'}')
    .add(b'%')
    .add(b'/');

/// Wrapper that escapes arguments for URL path segments.
pub(super) struct PathArg<A: Display>(pub(super) A);

/// Wrapper that escapes arguments for URL query segments.
pub(super) struct QueryArg<A: Display>(pub(super) A);

impl<A: Display> Display for PathArg<A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        // if we put values into the path we need to url encode them.  However
        // special care needs to be taken for any slash character or path
        // segments that would end up as ".." or "." for security reasons.
        // Since we cannot handle slashes there we just replace them with the
        // unicode replacement character as a quick workaround.  This will
        // typically result in 404s from the server.
        let mut val = format!("{}", self.0).replace('/', "\u{fffd}");
        if val == ".." || val == "." {
            val = "\u{fffd}".into();
        }
        utf8_percent_encode(&val, &PATH_SEGMENT_ENCODE_SET).fmt(f)
    }
}

impl<A: Display> Display for QueryArg<A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        utf8_percent_encode(&format!("{}", self.0), &QUERY_ENCODE_SET).fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("/")]
    #[case(".")]
    #[case("..")]
    fn test_path_arg_replacement_cases(#[case] input: &str) {
        assert_eq!(
            format!("{}", PathArg(input)),
            format!("{}", utf8_percent_encode("\u{fffd}", CONTROLS)),
            "case \"{input}\" failed"
        );
    }

    #[rstest]
    #[case(" ", "%20")]
    #[case("\"", "%22")]
    #[case("#", "%23")]
    #[case("<", "%3C")]
    #[case(">", "%3E")]
    #[case("+", "%2B")]
    #[case(" \"#<>+", "%20%22%23%3C%3E%2B")]
    #[case("1 + 3 < 4", "1%20%2B%203%20%3C%204")]
    fn test_path_arg_percent_encode(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(
            format!("{}", PathArg(input)),
            expected,
            "case \"{input}\" failed"
        );
    }
}
