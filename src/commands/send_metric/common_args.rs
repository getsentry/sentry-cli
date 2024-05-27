use crate::utils::value_parsers;
use anyhow::{anyhow, Result};
use clap::command;
use clap::Args;
use sentry::metrics::MetricStr;
use std::str::FromStr;

/// Arguments for send-metric subcommands using float as value type and no default value.
#[derive(Args)]
pub(super) struct FloatValueMetricArgs {
    #[command(flatten)]
    pub(super) common: CommonMetricArgs,

    #[arg(short, long, help = "Metric value, any finite 64 bit float.")]
    pub(super) value: f64,
}

/// Common arguments for all send-metric subcommands.
#[derive(Args)]
pub(super) struct CommonMetricArgs {
    #[arg(short, long)]
    #[arg(help = "The name of the metric, identifying it in Sentry.")]
    pub(super) name: MetricName,

    #[arg(short, long)]
    #[arg(
        help = "Any custom unit. You can have multiple metrics with the same name but different \
        units."
    )]
    pub(super) unit: Option<String>,

    #[arg(short, long, value_delimiter=',', value_name = "KEY:VALUE", num_args = 1..)]
    #[arg(value_parser=value_parsers::kv_parser)]
    #[arg(
        help = "Metric tags as key:value pairs. Tags allow you to add dimensions to your metrics \
        and can be filtered or grouped by in Sentry."
    )]
    pub(super) tags: Vec<(String, String)>,
}

#[derive(Clone)]
pub(super) struct MetricName(String);

impl FromStr for MetricName {
    type Err = anyhow::Error;

    /// Metric name must start with an alphabetic character.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.chars()
            .next()
            .ok_or_else(|| anyhow!("metric name cannot be empty"))?
            .is_ascii_alphabetic()
        {
            Ok(MetricName(s.to_string()))
        } else {
            Err(anyhow!(
                "metric name must start with an alphabetic character"
            ))
        }
    }
}

impl From<MetricName> for MetricStr {
    fn from(name: MetricName) -> MetricStr {
        MetricStr::from(name.0)
    }
}
