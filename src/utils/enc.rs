use std::str;
use std::borrow::Cow;

use uchardet::detect_encoding_name;
use encoding::DecoderTrap;
use encoding::label::encoding_from_whatwg_label;

use prelude::*;


// Decodes bytes from an unknown encoding
pub fn decode_unknown_string(bytes: &[u8]) -> Result<Cow<str>> {
    if let Ok(s) = str::from_utf8(bytes) {
        Ok(Cow::Borrowed(s))
    } else {
        if_chain! {
            if let Ok(enc) = detect_encoding_name(bytes);
            if let Some(enc) = encoding_from_whatwg_label(&enc);
            if let Ok(s) = enc.decode(bytes, DecoderTrap::Replace);
            then {
                Ok(Cow::Owned(s))
            } else {
                fail!("unknown encoding for string");
            }
        }
    }
}
