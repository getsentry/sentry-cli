use super::{
    normalized_name::NormalizedName, normalized_tags::NormalizedTags,
    normalized_unit::NormalizedUnit, types::MetricType, values::MetricValue,
};
use crate::commands::send_metric::common_args::CommonMetricArgs;
use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct NormalizedPayload<'a> {
    name: NormalizedName<'a>,
    value: MetricValue,
    metric_type: MetricType,
    unit: NormalizedUnit<'a>,
    tags: NormalizedTags,
}

impl<'a> NormalizedPayload<'a> {
    pub fn from_cli_args(
        common_args: &'a CommonMetricArgs,
        value: MetricValue,
        metric_type: MetricType,
    ) -> Self {
        Self {
            name: NormalizedName::from(&common_args.name),
            value,
            metric_type,
            unit: NormalizedUnit::from(&common_args.unit),
            tags: NormalizedTags::from(common_args.tags.as_slice()).with_default_tags(),
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let data = format!(
            "{}@{}:{}|{}|#{}|T{}",
            self.name, self.unit, self.value, self.metric_type, self.tags, timestamp
        );
        Ok(data.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::{
        commands::send_metric::common_args::{CommonMetricArgs, MetricName},
        config::Config,
        utils::metrics::{
            normalized_payload::NormalizedPayload, types::MetricType, values::MetricValue,
        },
    };
    use regex::Regex;

    #[test]
    fn test_from_cli_args_and_to_bytes() {
        Config::from_cli_config().unwrap().bind_to_process();
        let common_args = CommonMetricArgs {
            name: MetricName::from_str("nöme").unwrap(),
            unit: Some("möb".to_string()),
            tags: vec![("atagö".to_string(), "aval|ö".to_string())],
        };
        let expected = Regex::new(
            r"^n_me@mb:1\|s\|#atag:aval\\u\{7c}ö,environment:production,release:.+\|T\d{10}$",
        )
        .unwrap();

        let bytes =
            NormalizedPayload::from_cli_args(&common_args, MetricValue::UInt(1), MetricType::Set)
                .to_bytes()
                .unwrap();
        let actual = String::from_utf8_lossy(&bytes);

        assert!(expected.is_match(&actual));
    }
}
