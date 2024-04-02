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
pub struct PathArg<A: Display>(pub A);

/// Wrapper that escapes arguments for URL query segments.
pub struct QueryArg<A: Display>(pub A);

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
