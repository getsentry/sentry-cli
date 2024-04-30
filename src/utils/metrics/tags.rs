use crate::{config::Config, utils::releases};
use itertools::Itertools;
use regex::Regex;
use std::collections::HashMap;

pub(super) struct NormalizedTags {
    tags: HashMap<String, String>,
}

impl<T> From<T> for NormalizedTags
where
    T: IntoIterator<Item = (String, String)>,
{
    fn from(tags: T) -> Self {
        Self {
            tags: tags.into_iter().collect(),
        }
        .with_default_tags()
        .normalized()
    }
}

impl NormalizedTags {
    fn with_default_tags(mut self) -> Self {
        if let Ok(release) = releases::detect_release_name() {
            self.tags.entry("release".to_string()).or_insert(release);
        }
        self.tags.entry("environment".to_string()).or_insert(
            Config::current()
                .get_environment()
                .unwrap_or("production".to_string()),
        );
        self
    }

    fn normalized(mut self) -> Self {
        self.tags = self
            .tags
            .iter()
            .filter(|(k, v)| {
                !self.normalize_tag_key(k).is_empty() && !self.normalize_tag_value(v).is_empty()
            })
            .map(|(k, v)| (self.normalize_tag_key(k), self.normalize_tag_value(v)))
            .collect();
        self
    }

    fn normalize_tag_key(&self, key: &str) -> String {
        Regex::new(r"[^a-zA-Z0-9_\-./]")
            .expect("Tag normalization regex should compile")
            .replace_all(key, "")
            .to_string()
    }

    fn normalize_tag_value(&self, value: &str) -> String {
        value
            .replace('\\', "\\\\")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
            .replace('|', "\\u{7c}")
            .replace(',', "\\u{2c}")
    }
}

impl std::fmt::Display for NormalizedTags {
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
