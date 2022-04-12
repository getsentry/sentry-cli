use std::borrow::Cow;
use std::str;

use anyhow::Result;
use chardet::detect;
use encoding::label::encoding_from_whatwg_label;
use encoding::DecoderTrap;
use if_chain::if_chain;

#[derive(thiserror::Error, Debug)]
#[error("unknown encoding for string")]
pub struct UnknownEncodingError;

// Decodes bytes from an unknown encoding
pub fn decode_unknown_string(bytes: &[u8]) -> Result<Cow<'_, str>> {
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
                Err(UnknownEncodingError.into())
            }
        }
    }
}
