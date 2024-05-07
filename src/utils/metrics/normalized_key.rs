use regex::Regex;
use std::borrow::Cow;

pub(super) struct NormalizedKey<'a> {
    key: Cow<'a, str>,
}

impl<'a> From<&'a str> for NormalizedKey<'a> {
    fn from(key: &'a str) -> Self {
        Self {
            key: Regex::new(r"[^a-zA-Z0-9_\-.]")
                .expect("Regex should compile")
                .replace_all(key, "_"),
        }
    }
}

impl std::fmt::Display for NormalizedKey<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.key)
    }
}
