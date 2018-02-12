use std::str;
use std::borrow::Cow;

use chardet::detect;
use encoding::DecoderTrap;
use encoding::label::encoding_from_whatwg_label;

use errors::{ErrorKind, Result};

// Decodes bytes from an unknown encoding
pub fn decode_unknown_string(bytes: &[u8]) -> Result<Cow<str>> {
    if let Ok(s) = str::from_utf8(bytes) {
        Ok(Cow::Borrowed(s))
    } else {
        let (label, confidence, _) = detect(bytes);
        if_chain! {
            if confidence >= 0.5;
            if let Some(enc) = encoding_from_whatwg_label(&label);
            if let Ok(s) = enc.decode(bytes, DecoderTrap::Replace);
            then {
                Ok(Cow::Owned(s))
            } else {
                println_stderr!("unknown encoding for string");
                return Err(ErrorKind::QuietExit(1).into());
            }
        }
    }
}
