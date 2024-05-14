use crate::commands::send_metric::common_args::MetricName;
use regex::Regex;
use std::borrow::Cow;

pub(super) struct NormalizedName<'a> {
    name: Cow<'a, str>,
}

impl<'a> From<&'a MetricName> for NormalizedName<'a> {
    fn from(name: &'a MetricName) -> Self {
        Self {
            name: Regex::new(r"[^a-zA-Z0-9_\-.]")
                .expect("Regex should compile")
                .replace_all(name.as_str(), "_"),
        }
    }
}

impl std::fmt::Display for NormalizedName<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        commands::send_metric::common_args::MetricName,
        utils::metrics::normalized_name::NormalizedName,
    };
    use std::str::FromStr;

    #[test]
    fn test_from() {
        let expected = "aA1_-.____________";

        let actual =
            NormalizedName::from(&MetricName::from_str("aA1_-./+ö{😀\n\t\r\\| ,").unwrap())
                .to_string();

        assert_eq!(expected, actual);
    }
}
