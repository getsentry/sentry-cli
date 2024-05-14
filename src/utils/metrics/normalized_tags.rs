use crate::{config::Config, utils::releases};
use itertools::Itertools;
use regex::Regex;
use std::{borrow::Cow, cmp::min, collections::HashMap};

pub(super) struct NormalizedTags<'a> {
    tags: HashMap<Cow<'a, str>, String>,
}

impl<'a> From<&'a Vec<(String, String)>> for NormalizedTags<'a> {
    fn from(tags: &'a Vec<(String, String)>) -> Self {
        Self {
            tags: tags
                .iter()
                .map(|(k, v)| (Self::normalize_key(k), Self::normalize_value(v)))
                .filter(|(k, v)| !v.is_empty() && !k.is_empty())
                .collect(),
        }
    }
}

impl NormalizedTags<'_> {
    fn normalize_key(key: &str) -> Cow<str> {
        Regex::new(r"[^a-zA-Z0-9_\-./]")
            .expect("Tag normalization regex should compile")
            .replace_all(&key[..min(key.len(), 32)], "")
    }

    fn normalize_value(value: &str) -> String {
        value[..min(value.len(), 200)]
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

#[cfg(test)]
mod test {
    use std::env;

    use crate::config::Config;

    use super::NormalizedTags;

    #[test]
    fn test_replacement_characters() {
        let tags = vec![
            ("a\na", "a\na"),
            ("b\rb", "b\rb"),
            ("c\tc", "c\tc"),
            ("d\\d", "d\\d"),
            ("e|e", "e|e"),
            ("f,f", "f,f"),
        ]
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        let expected = "aa:a\\na,bb:b\\rb,cc:c\\tc,dd:d\\\\d,ee:e\\u{7c}e,ff:f\\u{2c}f";

        let actual = NormalizedTags::from(&tags).to_string();

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_empty_tags() {
        let tags = vec![("+", "a"), ("a", ""), ("", "a"), ("", "")]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let expected = "";

        let actual = NormalizedTags::from(&tags).to_string();

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_special_characters() {
        let tags = vec![("aA1_-./+ö{ 😀", "aA1_-./+ö{ 😀")]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let expected = "aA1_-./:aA1_-./+ö{ 😀";

        let actual = NormalizedTags::from(&tags).to_string();

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_add_default_tags() {
        Config::from_cli_config().unwrap().bind_to_process();
        env::set_var("SOURCE_VERSION", "my-release");
        let expected = "environment:production,release:my-release";

        let actual = NormalizedTags::from(&Vec::new())
            .with_default_tags()
            .to_string();

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_override_default_tags() {
        Config::from_cli_config().unwrap().bind_to_process();
        env::set_var("SOURCE_VERSION", "my-release");
        let expected = "environment:env_override,release:release_override";

        let actual = NormalizedTags::from(&vec![
            ("release".to_string(), "release_override".to_string()),
            ("environment".to_string(), "env_override".to_string()),
        ])
        .with_default_tags()
        .to_string();

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_tag_lengths() {
        let expected = "abcdefghijklmnopqrstuvwxyzabcdef:abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqr";

        let actual = NormalizedTags::from(&vec![
            ("abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz".to_string(), 
            "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz".to_string()),
        ])
        .to_string();

        assert_eq!(expected, actual);
    }
}
