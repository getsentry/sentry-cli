use std::str;
use std::borrow::Cow;

use chardet::detect;

use prelude::*;


// Decodes bytes from an unknown encoding
pub fn decode_unknown_string(bytes: &[u8]) -> Result<Cow<str>> {
    if let Ok(s) = str::from_utf8(bytes) {
        Ok(Cow::Borrowed(s))
    } else {
        let (enc, confidence, _) = detect(bytes);
        if confidence < 0.5 {
            fail!("unknown encoding for string");
        } else {
            Ok(Cow::Owned(enc))
        }
    }
}
