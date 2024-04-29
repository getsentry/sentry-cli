use super::tags::NormalizedTags;
use crate::commands::send_metric::subcommands::{CommonMetricArgs, SendMetricSubcommand};
use anyhow::Result;
use regex::Regex;
use std::io::Write;
use std::string::FromUtf8Error;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct MetricsPayload {
    payload: Vec<u8>,
}

impl MetricsPayload {
    /// Creates a normalized MetricsPayload, consuming the subcommand
    pub fn from_subcommand(command: SendMetricSubcommand) -> Result<Self> {
        let (metric_value, metric_type) = (command.value(), command.metric_type());
        let common_args = CommonMetricArgs::from(command);
        Self {
            payload: Vec::new(),
        }
        .with_name(&common_args.key)?
        .with_unit(common_args.unit)?
        .with_value(metric_value)?
        .with_type(metric_type)?
        .with_tags(common_args.tags)?
        .with_timestamp()
    }

    fn with_name(mut self, name: &str) -> Result<Self> {
        let safe_name = Regex::new(r"[^a-zA-Z0-9_\-.]")?.replace_all(name, "_");
        write!(self.payload, "{safe_name}")?;
        Ok(self)
    }

    fn with_unit(mut self, unit: Option<String>) -> Result<Self> {
        if let Some(unit) = unit {
            let safe_unit = Regex::new(r"[^a-zA-Z0-9_]")?.replace_all(&unit, "");
            if !safe_unit.is_empty() {
                write!(self.payload, "@{safe_unit}")?;
            }
        }
        Ok(self)
    }

    fn with_value(mut self, value: f64) -> Result<Self> {
        write!(self.payload, ":{}", value)?;
        Ok(self)
    }

    fn with_type(mut self, metric_type: char) -> Result<Self> {
        write!(self.payload, "|{}", metric_type)?;
        Ok(self)
    }

    fn with_tags<T>(mut self, tags: T) -> Result<Self>
    where
        T: IntoIterator<Item = (String, String)>,
    {
        write!(self.payload, "|#{}", NormalizedTags::from(tags))?;
        Ok(self)
    }

    fn with_timestamp(mut self) -> Result<Self> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        write!(self.payload, "|T{timestamp}",)?;
        Ok(self)
    }

    pub fn to_string(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.payload.clone())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.payload.clone()
    }
}
