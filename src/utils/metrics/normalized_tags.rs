use crate::{config::Config, utils::releases};
use itertools::Itertools;
use regex::Regex;
use std::{borrow::Cow, collections::HashMap};

pub(super) struct NormalizedTags<'a> {
    tags: HashMap<Cow<'a, str>, String>,
}

impl<'a> From<&'a Vec<(String, String)>> for NormalizedTags<'a> {
    fn from(tags: &'a Vec<(String, String)>) -> Self {
        Self {
            tags: tags
                .iter()
                .map(|(k, v)| (Self::normalize_key(&k), Self::normalize_value(&v)))
                .filter(|(k, v)| !v.is_empty() && !k.is_empty())
                .collect(),
        }
    }
}

impl NormalizedTags<'_> {
    fn normalize_key(key: &str) -> Cow<str> {
        Regex::new(r"[^a-zA-Z0-9_\-./]")
            .expect("Tag normalization regex should compile")
            .replace_all(key, "")
    }

    fn normalize_value(value: &str) -> String {
        value
            .replace('\\', "\\\\")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
            .replace('|', "\\u{7c}")
            .replace(',', "\\u{2c}")
    }

    pub(super) fn with_default_tags(mut self) -> Self {
        if let Ok(release) = releases::detect_release_name() {
            self.tags
                .entry(Cow::Borrowed("release"))
                .or_insert(Self::normalize_value(&release));
        }
        self.tags
            .entry(Cow::Borrowed("environment"))
            .or_insert(Self::normalize_value(
                &Config::current()
                    .get_environment()
                    .unwrap_or("production".to_string()),
            ));
        self
    }
}

impl std::fmt::Display for NormalizedTags<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = self
            .tags
            .iter()
            .map(|(k, v)| format!("{}:{}", k, v))
            .sorted()
            .join(",");
        write!(f, "{res}")
    }
}
