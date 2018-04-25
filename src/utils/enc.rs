use std::str;
use std::borrow::Cow;

use failure::Error;
use chardet::detect;
use encoding::DecoderTrap;
use encoding::label::encoding_from_whatwg_label;

#[derive(Fail, Debug)]
#[fail(display = "unknown encoding for string")]
pub struct UnknownEncodingError;

// Decodes bytes from an unknown encoding
pub fn decode_unknown_string(bytes: &[u8]) -> Result<Cow<str>, Error> {
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
                return Err(UnknownEncodingError.into());
            }
        }
    }
}
