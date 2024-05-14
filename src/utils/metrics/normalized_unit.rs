use regex::Regex;
use std::borrow::Cow;

pub(super) struct NormalizedUnit<'a> {
    unit: Cow<'a, str>,
}

impl<'a> From<&'a Option<String>> for NormalizedUnit<'a> {
    fn from(unit: &'a Option<String>) -> Self {
        let safe_unit = match unit {
            Some(unit) => Regex::new(r"[^a-zA-Z0-9_]")
                .expect("Regex should compile")
                .replace_all(unit, ""),
            None => Cow::from(""),
        };
        Self {
            unit: if safe_unit.is_empty() {
                Cow::from("none")
            } else {
                safe_unit
            },
        }
    }
}

impl std::fmt::Display for NormalizedUnit<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.unit)
    }
}

#[cfg(test)]
mod test {
    use crate::utils::metrics::normalized_unit::NormalizedUnit;

    #[test]
    fn test_from() {
        let unit = Some("aA1_-./+ö{😀\n\t\r\\| ,".to_string());
        let expected = "aA1_";

        let actual = NormalizedUnit::from(&unit).to_string();

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_from_empty() {
        let unit = Some("".to_string());
        let expected = "none";

        let actual = NormalizedUnit::from(&unit).to_string();

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_from_empty_after_normalization() {
        let unit = Some("+".to_string());
        let expected = "none";

        let actual = NormalizedUnit::from(&unit).to_string();

        assert_eq!(expected, actual);
    }
}
