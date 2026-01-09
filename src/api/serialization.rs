//! This module contains some custom serialization logic for the API.
use std::sync::LazyLock;

use regex::Regex;
use serde::ser::SerializeSeq as _;
use serde::{Serialize, Serializer};

/// A container for either a numeric ID or an alphanumeric slug.
///
/// IDs are serialized as integers, while slugs are serialized as strings.
#[derive(Serialize)]
#[serde(untagged)]
enum IdSlug<'s> {
    Id(i64),
    Slug(&'s str),
}

/// Serializes a sequence of strings, which may contain either numeric IDs or alphanumeric slugs.
///
/// We check each element in the sequence. If the element only contains digits and can be parsed as a 64-bit signed integer,
/// we consider the value to be an ID. Otherwise, we consider the value to be a slug.
///
/// IDs are serialized as integers, while slugs are serialized as strings.
pub fn serialize_id_slug_list<I, S>(list: I, serializer: S) -> Result<S::Ok, S::Error>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
    S: Serializer,
{
    let mut seq = serializer.serialize_seq(None)?;
    for item in list {
        let item = item.as_ref();
        let id_slug = IdSlug::from(&item);
        seq.serialize_element(&id_slug)?;
    }
    seq.end()
}

impl<'a, S> From<&'a S> for IdSlug<'a>
where
    S: AsRef<str>,
{
    /// Convert from a string reference to an IdSlug.
    ///
    /// If the string contains only digits and can be parsed as a 64-bit signed integer,
    /// we consider the value to be an ID. Otherwise, we consider the value to be a slug.
    fn from(value: &'a S) -> Self {
        /// Project ID regex
        ///
        /// Project IDs always contain only digits.
        static PROJECT_ID_REGEX: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\d+$").expect("regex is valid"));

        let value = value.as_ref();

        PROJECT_ID_REGEX
            .is_match(value)
            .then(|| value.parse().ok().map(IdSlug::Id))
            .flatten()
            .unwrap_or(IdSlug::Slug(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test struct which serializes with serialize_id_slug_list
    #[derive(Serialize)]
    struct IdSlugListSerializerTest<const N: usize> {
        #[serde(serialize_with = "serialize_id_slug_list")]
        value: [&'static str; N],
    }

    #[test]
    fn test_serialize_id_slug_list_empty() {
        let to_serialize = IdSlugListSerializerTest { value: [] };

        let serialized = serde_json::to_string(&to_serialize).unwrap();
        let expected = serde_json::json!({ "value": [] }).to_string();

        assert_eq!(serialized, expected)
    }

    #[test]
    fn test_serialize_id_slug_list_single_id() {
        let to_serialize = IdSlugListSerializerTest { value: ["123"] };

        let serialized = serde_json::to_string(&to_serialize).unwrap();
        let expected = serde_json::json!({ "value": [123] }).to_string();

        assert_eq!(serialized, expected)
    }

    #[test]
    fn test_serialize_id_slug_list_single_slug() {
        let to_serialize = IdSlugListSerializerTest { value: ["abc"] };

        let serialized = serde_json::to_string(&to_serialize).unwrap();
        let expected = serde_json::json!({ "value": ["abc"] }).to_string();

        assert_eq!(serialized, expected)
    }

    #[test]
    fn test_serialize_id_slug_list_multiple_ids_and_slugs() {
        let to_serialize = IdSlugListSerializerTest {
            value: ["123", "abc", "456", "whatever"],
        };

        let serialized = serde_json::to_string(&to_serialize).unwrap();
        let expected = serde_json::json!({ "value": [123, "abc", 456, "whatever"] }).to_string();

        assert_eq!(serialized, expected)
    }

    /// Slugs of "-0" are possible. This test ensures that we serialize "-0" as a slug,
    /// rather than as an ID 0.
    #[test]
    fn test_serialize_id_slug_minus_zero_edge_case() {
        let to_serialize = IdSlugListSerializerTest { value: ["-0"] };

        let serialized = serde_json::to_string(&to_serialize).unwrap();
        let expected = serde_json::json!({ "value": ["-0"] }).to_string();

        assert_eq!(serialized, expected)
    }
}
