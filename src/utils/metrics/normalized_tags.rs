use crate::{config::Config, utils::releases};
use itertools::Itertools;
use regex::Regex;
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;

pub(super) struct NormalizedTags {
    tags: HashMap<String, String>,
}

impl From<&[(String, String)]> for NormalizedTags {
    fn from(tags: &[(String, String)]) -> Self {
        Self {
            tags: tags
                .iter()
                .map(|(k, v)| (Self::normalize_key(k), Self::normalize_value(v)))
                .filter(|(k, v)| !v.is_empty() && !k.is_empty())
                .collect(),
        }
    }
}

impl NormalizedTags {
    fn normalize_key(key: &str) -> String {
        Regex::new(r"[^a-zA-Z0-9_\-./]")
            .expect("Tag normalization regex should compile")
            .replace_all(&key.graphemes(true).take(32).collect::<String>(), "")
            .to_string()
    }

    fn normalize_value(value: &str) -> String {
        value
            .graphemes(true)
            .take(200)
            .collect::<String>()
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
                .entry("release".to_string())
                .or_insert(Self::normalize_value(&release));
        }
        self.tags
            .entry("environment".to_string())
            .or_insert(Self::normalize_value(
                &Config::current()
                    .get_environment()
                    .unwrap_or("production".to_string()),
            ));
        self
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

#[cfg(test)]
mod test {
    use std::env;

    use crate::config::Config;

    use super::NormalizedTags;

    #[test]
    fn test_replacement_characters() {
        let tags: Vec<(String, String)> = [
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

        let actual = NormalizedTags::from(tags.as_slice()).to_string();

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_empty_tags() {
        let tags: Vec<(String, String)> = [("+", "a"), ("a", ""), ("", "a"), ("", "")]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let expected = "";

        let actual = NormalizedTags::from(tags.as_slice()).to_string();

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_special_characters() {
        let tags: Vec<(String, String)> = [("aA1_-./+ö{ 😀", "aA1_-./+ö{ 😀")]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let expected = "aA1_-./:aA1_-./+ö{ 😀";

        let actual = NormalizedTags::from(tags.as_slice()).to_string();

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_add_default_tags() {
        Config::from_cli_config().unwrap().bind_to_process();
        env::set_var("SOURCE_VERSION", "my-release");
        let expected = "environment:production,release:my-release";

        let actual = NormalizedTags::from(Vec::new().as_slice())
            .with_default_tags()
            .to_string();

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_override_default_tags() {
        Config::from_cli_config().unwrap().bind_to_process();
        env::set_var("SOURCE_VERSION", "my-release");
        let expected = "environment:env_override,release:release_override";

        let actual = NormalizedTags::from(
            vec![
                ("release".to_string(), "release_override".to_string()),
                ("environment".to_string(), "env_override".to_string()),
            ]
            .as_slice(),
        )
        .with_default_tags()
        .to_string();

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_tag_lengths() {
        let expected = "abcdefghijklmnopqrstuvwxyzabcde:abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopq🙂";

        let actual = NormalizedTags::from(vec![
            ("abcdefghijklmnopqrstuvwxyzabcde🙂fghijklmnopqrstuvwxyz".to_string(), 
            "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopq🙂rstuvwxyz".to_string()),
        ].as_slice())
        .to_string();

        assert_eq!(expected, actual);
    }
}
