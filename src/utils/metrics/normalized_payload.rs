use super::{
    normalized_key::NormalizedKey, normalized_tags::NormalizedTags,
    normalized_unit::NormalizedUnit, types::MetricType, values::MetricValue,
};
use crate::commands::send_metric::common_args::CommonMetricArgs;
use anyhow::Result;
use std::{
    io::Write,
    time::{SystemTime, UNIX_EPOCH},
};

pub struct NormalizedPayload<'a> {
    key: NormalizedKey<'a>,
    value: MetricValue,
    metric_type: MetricType,
    unit: NormalizedUnit<'a>,
    tags: NormalizedTags<'a>,
}

impl<'a> NormalizedPayload<'a> {
    pub fn from_cli_args(
        common_args: &'a CommonMetricArgs,
        value: MetricValue,
        metric_type: MetricType,
    ) -> Self {
        Self {
            key: NormalizedKey::from(common_args.key.as_ref()),
            value,
            metric_type,
            unit: NormalizedUnit::from(&common_args.unit),
            tags: NormalizedTags::from(&common_args.tags).with_default_tags(),
        }
    }

    pub fn to_string(&self) -> Result<String> {
        Ok(String::from_utf8(self.to_bytes()?)?)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        write!(data, "{}", self.key)?;
        write!(data, "@{}", self.unit)?;
        write!(data, ":{}", self.value)?;
        write!(data, "|{}", self.metric_type)?;
        write!(data, "|#{}", self.tags)?;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        write!(data, "|T{timestamp}")?;
        Ok(data)
    }
}
